using System.Net.Http;
using System.Text;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class DuckDnsProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;

    public DuckDnsProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://www.duckdns.org/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.DuckDNS;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.DuckDnsToken))
        {
            return new ProviderResult(false, "DuckDNS Token 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.DuckDNS.Domain))
        {
            return new ProviderResult(false, "DuckDNS Domain 未填写");
        }

        var token = profile.Secrets.DuckDnsToken;
        var domain = profile.DuckDNS.Domain;

        var result = await UpdateDdnsRecordAsync(token, domain, ipAddress, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return new ProviderResult(false, $"更新 DNS 记录失败: {result.Message}");
        }

        return new ProviderResult(true, $"已更新 {recordType} 记录 -> {ipAddress}");
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.DuckDnsToken))
        {
            return new ProviderResult(false, "DuckDNS Token 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.DuckDNS.Domain))
        {
            return new ProviderResult(false, "DuckDNS Domain 未填写");
        }

        var token = profile.Secrets.DuckDnsToken;
        var domain = profile.DuckDNS.Domain;

        var testResult = await UpdateDdnsRecordAsync(token, domain, "127.0.0.1", cancellationToken).ConfigureAwait(false);
        if (!testResult.Success)
        {
            return new ProviderResult(false, $"测试失败: {testResult.Message}");
        }

        return new ProviderResult(true, "测试成功");
    }

    public void Dispose()
    {
        _httpClient.Dispose();
    }

    private async Task<(bool Success, string Message)> UpdateDdnsRecordAsync(string token, string domain, string ipAddress, CancellationToken cancellationToken)
    {
        var requestUrl = $"https://www.duckdns.org/update?domains={Uri.EscapeDataString(domain)}&token={Uri.EscapeDataString(token)}&ip={Uri.EscapeDataString(ipAddress)}";

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, requestUrl);
            using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
            var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return (false, $"HTTP {(int)response.StatusCode}");
            }

            if (!TryParseDuckDnsResponse(raw, out var errorMessage))
            {
                _logService.Log(LogLevel.Warning, $"DuckDNS API 返回失败: {errorMessage}");
                return (false, errorMessage);
            }

            return (true, "ok");
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return (false, "请求超时");
        }
        catch (HttpRequestException ex)
        {
            return (false, $"网络请求失败: {ex.Message}");
        }
        catch (Exception ex)
        {
            return (false, $"请求失败: {ex.Message}");
        }
    }

    private bool TryParseDuckDnsResponse(string raw, out string errorMessage)
    {
        errorMessage = string.Empty;

        try
        {
            var response = raw.Trim();
            if (response.Equals("OK", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            else if (response.Equals("KO", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "更新失败";
                return false;
            }
            else
            {
                errorMessage = response;
                return false;
            }
        }
        catch (Exception ex)
        {
            errorMessage = $"解析响应失败: {ex.Message}";
            return false;
        }
    }
}