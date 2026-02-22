using System.Collections.Concurrent;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class CloudflareProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;
    private readonly ConcurrentDictionary<string, string> _zoneIdCache = new(StringComparer.OrdinalIgnoreCase);

    public CloudflareProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://api.cloudflare.com/client/v4/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.Cloudflare;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.CloudflareApiToken))
        {
            return new ProviderResult(false, "Cloudflare Token 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Cloudflare.ZoneName))
        {
            return new ProviderResult(false, "Cloudflare ZoneName 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Hostname))
        {
            return new ProviderResult(false, "记录名未填写");
        }

        var zoneResult = await GetZoneIdAsync(profile.Cloudflare.ZoneName, profile.Secrets.CloudflareApiToken, cancellationToken).ConfigureAwait(false);
        if (!zoneResult.Success)
        {
            return new ProviderResult(false, zoneResult.Message);
        }

        var zoneId = zoneResult.ZoneId;
        var existingRecordResult = await FindRecordIdAsync(zoneId, profile.Hostname, recordType, profile.Secrets.CloudflareApiToken, cancellationToken).ConfigureAwait(false);
        if (!existingRecordResult.Success)
        {
            return new ProviderResult(false, existingRecordResult.Message);
        }

        var payload = new
        {
            type = recordType.ToString(),
            name = profile.Hostname,
            content = ipAddress,
            ttl = NormalizeTtl(profile.Cloudflare.Ttl),
            proxied = profile.Cloudflare.Proxied
        };

        var body = JsonSerializer.Serialize(payload);
        if (string.IsNullOrWhiteSpace(existingRecordResult.RecordId))
        {
            var createResult = await SendApiRequestAsync(
                profile.Secrets.CloudflareApiToken,
                HttpMethod.Post,
                $"zones/{zoneId}/dns_records",
                body,
                cancellationToken).ConfigureAwait(false);

            if (!createResult.Success)
            {
                return new ProviderResult(false, $"创建 DNS 记录失败: {createResult.Message}", createResult.Raw);
            }

            return new ProviderResult(true, $"已创建 {recordType} 记录 -> {ipAddress}", createResult.Raw);
        }

        var updateResult = await SendApiRequestAsync(
            profile.Secrets.CloudflareApiToken,
            HttpMethod.Put,
            $"zones/{zoneId}/dns_records/{existingRecordResult.RecordId}",
            body,
            cancellationToken).ConfigureAwait(false);

        if (!updateResult.Success)
        {
            return new ProviderResult(false, $"更新 DNS 记录失败: {updateResult.Message}", updateResult.Raw);
        }

        return new ProviderResult(true, $"已更新 {recordType} 记录 -> {ipAddress}", updateResult.Raw);
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.CloudflareApiToken) || string.IsNullOrWhiteSpace(profile.Cloudflare.ZoneName))
        {
            return new ProviderResult(false, "请先填写 Cloudflare Token 和 ZoneName");
        }

        var result = await GetZoneIdAsync(profile.Cloudflare.ZoneName, profile.Secrets.CloudflareApiToken, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return new ProviderResult(false, $"测试失败: {result.Message}");
        }

        return new ProviderResult(true, $"测试成功，Zone ID: {result.ZoneId}");
    }

    public void Dispose()
    {
        _httpClient.Dispose();
    }

    private async Task<(bool Success, string ZoneId, string Message)> GetZoneIdAsync(string zoneName, string token, CancellationToken cancellationToken)
    {
        if (_zoneIdCache.TryGetValue(zoneName, out var cachedZoneId) && !string.IsNullOrWhiteSpace(cachedZoneId))
        {
            return (true, cachedZoneId, "ok");
        }

        var result = await SendApiRequestAsync(
            token,
            HttpMethod.Get,
            $"zones?name={Uri.EscapeDataString(zoneName)}&per_page=1",
            null,
            cancellationToken).ConfigureAwait(false);

        if (!result.Success)
        {
            return (false, string.Empty, result.Message);
        }

        var node = JsonNode.Parse(result.Raw);
        var zoneId = node?["result"]?.AsArray().FirstOrDefault()?["id"]?.GetValue<string>();
        if (string.IsNullOrWhiteSpace(zoneId))
        {
            return (false, string.Empty, "未找到 Zone ID，请确认 ZoneName");
        }

        _zoneIdCache[zoneName] = zoneId;
        return (true, zoneId, "ok");
    }

    private async Task<(bool Success, string RecordId, string Message)> FindRecordIdAsync(
        string zoneId,
        string hostname,
        DnsRecordType recordType,
        string token,
        CancellationToken cancellationToken)
    {
        var result = await SendApiRequestAsync(
            token,
            HttpMethod.Get,
            $"zones/{zoneId}/dns_records?type={recordType}&name={Uri.EscapeDataString(hostname)}&per_page=1",
            null,
            cancellationToken).ConfigureAwait(false);

        if (!result.Success)
        {
            return (false, string.Empty, result.Message);
        }

        var node = JsonNode.Parse(result.Raw);
        var recordId = node?["result"]?.AsArray().FirstOrDefault()?["id"]?.GetValue<string>() ?? string.Empty;
        return (true, recordId, "ok");
    }

    private async Task<(bool Success, string Message, string Raw)> SendApiRequestAsync(
        string token,
        HttpMethod method,
        string relativePath,
        string? body,
        CancellationToken cancellationToken)
    {
        using var request = new HttpRequestMessage(method, relativePath);
        request.Headers.TryAddWithoutValidation("Authorization", $"Bearer {token}");

        if (!string.IsNullOrWhiteSpace(body))
        {
            request.Content = new StringContent(body, Encoding.UTF8, "application/json");
        }

        using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
        var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode)
        {
            return (false, $"HTTP {(int)response.StatusCode}", raw);
        }

        if (!TryParseCloudflareEnvelope(raw, out _, out var errorMessage))
        {
            _logService.Log(LogLevel.Warning, $"Cloudflare API 返回失败: {errorMessage}");
            return (false, errorMessage, raw);
        }

        return (true, "ok", raw);
    }

    private static bool TryParseCloudflareEnvelope(string raw, out JsonNode? root, out string error)
    {
        error = string.Empty;
        root = null;

        try
        {
            root = JsonNode.Parse(raw);
            var success = root?["success"]?.GetValue<bool>() ?? false;
            if (success)
            {
                return true;
            }

            var errors = root?["errors"]?.AsArray();
            if (errors is not null && errors.Count > 0)
            {
                error = string.Join("; ", errors.Select(x => x?["message"]?.GetValue<string>() ?? "未知错误"));
            }
            else
            {
                error = "Cloudflare API success=false";
            }

            return false;
        }
        catch (Exception ex)
        {
            error = $"Cloudflare 响应解析失败: {ex.Message}";
            return false;
        }
    }

    private static int NormalizeTtl(int ttl)
    {
        if (ttl <= 1)
        {
            return 1;
        }

        return Math.Clamp(ttl, 60, 86400);
    }
}
