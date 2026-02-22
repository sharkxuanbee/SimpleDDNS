﻿using System.Net.Http;
using System.Text;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Core.Services;

namespace SimpleDDNS.Providers.Providers;

public sealed class GenericHttpProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;

    public GenericHttpProvider()
    {
        _httpClient = new HttpClient
        {
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.GenericHttp;

    public Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        return SendAsync(profile, recordType.ToString(), ipAddress, cancellationToken);
    }

    public Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        var testIp = profile.EnableIPv6 && !profile.EnableIPv4 ? "2001:db8::1" : "198.51.100.10";
        return SendAsync(profile, "TEST", testIp, cancellationToken);
    }

    public void Dispose()
    {
        _httpClient.Dispose();
    }

    private async Task<ProviderResult> SendAsync(DdnsProfile profile, string recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.GenericHttp.UrlTemplate))
        {
            return new ProviderResult(false, "URL 模板未填写");
        }

        var values = BuildPlaceholders(profile, recordType, ipAddress);
        var url = TemplateRenderer.Render(profile.GenericHttp.UrlTemplate, values);
        if (!Uri.TryCreate(url, UriKind.Absolute, out var requestUri))
        {
            return new ProviderResult(false, $"URL 无效: {url}");
        }

        if (requestUri.Scheme != Uri.UriSchemeHttp && requestUri.Scheme != Uri.UriSchemeHttps)
        {
            return new ProviderResult(false, "仅支持 HTTP/HTTPS 协议");
        }

        var method = profile.GenericHttp.Method == HttpUpdateMethod.Post ? HttpMethod.Post : HttpMethod.Get;
        using var request = new HttpRequestMessage(method, requestUri);

        foreach (var header in profile.Secrets.GenericHeaders)
        {
            if (string.IsNullOrWhiteSpace(header.Key))
            {
                continue;
            }

            request.Headers.TryAddWithoutValidation(header.Key, TemplateRenderer.Render(header.Value, values));
        }

        if (method == HttpMethod.Post)
        {
            var bodyTemplate = string.IsNullOrWhiteSpace(profile.GenericHttp.JsonBodyTemplate)
                ? "{\"ip\":\"{ip}\"}"
                : profile.GenericHttp.JsonBodyTemplate;
            
            // 为 JSON Body 准备转义后的值
            var jsonValues = values.ToDictionary(
                kv => kv.Key,
                kv => JsonEscape(kv.Value),
                StringComparer.OrdinalIgnoreCase);

            var payload = TemplateRenderer.Render(bodyTemplate, jsonValues);
            request.Content = new StringContent(payload, Encoding.UTF8, "application/json");
        }

        try
        {
            using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
            var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
            var shortened = raw.Length > 500 ? raw[..500] : raw;

            if (!response.IsSuccessStatusCode)
            {
                return new ProviderResult(false, $"HTTP {(int)response.StatusCode}: {shortened}", raw);
            }

            return new ProviderResult(true, $"HTTP {(int)response.StatusCode}: {shortened}", raw);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return new ProviderResult(false, "请求超时");
        }
        catch (HttpRequestException ex)
        {
            return new ProviderResult(false, $"网络请求失败: {ex.Message}");
        }
        catch (Exception ex)
        {
            return new ProviderResult(false, $"请求失败: {ex.Message}");
        }
    }

    private static Dictionary<string, string> BuildPlaceholders(DdnsProfile profile, string recordType, string ipAddress)
    {
        return new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["ip"] = ipAddress,
            ["hostname"] = profile.Hostname,
            ["type"] = recordType,
            ["ipv4"] = profile.RuntimeStatus.CurrentIPv4,
            ["ipv6"] = profile.RuntimeStatus.CurrentIPv6,
            ["timestamp"] = DateTimeOffset.Now.ToString("O")
        };
    }

    private static string JsonEscape(string value)
    {
        var json = System.Text.Json.JsonSerializer.Serialize(value);
        if (json.Length >= 2 && json.StartsWith('"') && json.EndsWith('"'))
        {
            return json.Substring(1, json.Length - 2);
        }
        return value;
    }
}
