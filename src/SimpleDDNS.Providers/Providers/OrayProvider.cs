using System.Net.Http;
using System.Text;
using System.Text.Json.Nodes;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class OrayProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;

    public OrayProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://ddns.oray.com/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.Oray;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.OrayUsername))
        {
            return new ProviderResult(false, "Oray Username 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.OrayPassword))
        {
            return new ProviderResult(false, "Oray Password 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Oray.Domain))
        {
            return new ProviderResult(false, "Oray Domain 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Oray.SubDomain))
        {
            return new ProviderResult(false, "Oray SubDomain 未填写");
        }

        var username = profile.Secrets.OrayUsername;
        var password = profile.Secrets.OrayPassword;
        var domain = profile.Oray.Domain;
        var subDomain = profile.Oray.SubDomain;
        var hostname = $"{subDomain}.{domain}";

        var result = await UpdateDdnsRecordAsync(username, password, hostname, ipAddress, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return new ProviderResult(false, $"更新 DNS 记录失败: {result.Message}");
        }

        return new ProviderResult(true, $"已更新 {recordType} 记录 -> {ipAddress}");
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.OrayUsername))
        {
            return new ProviderResult(false, "Oray Username 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.OrayPassword))
        {
            return new ProviderResult(false, "Oray Password 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Oray.Domain))
        {
            return new ProviderResult(false, "Oray Domain 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Oray.SubDomain))
        {
            return new ProviderResult(false, "Oray SubDomain 未填写");
        }

        var username = profile.Secrets.OrayUsername;
        var password = profile.Secrets.OrayPassword;
        var domain = profile.Oray.Domain;
        var subDomain = profile.Oray.SubDomain;
        var hostname = $"{subDomain}.{domain}";

        var testResult = await UpdateDdnsRecordAsync(username, password, hostname, "127.0.0.1", cancellationToken).ConfigureAwait(false);
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

    private async Task<(bool Success, string Message)> UpdateDdnsRecordAsync(string username, string password, string hostname, string ipAddress, CancellationToken cancellationToken)
    {
        var requestUrl = $"https://ddns.oray.com/v2/domain/update?hostname={Uri.EscapeDataString(hostname)}&ip={Uri.EscapeDataString(ipAddress)}";
        var credentials = Convert.ToBase64String(Encoding.ASCII.GetBytes($"{username}:{password}"));

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, requestUrl);
            request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Basic", credentials);
            using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
            var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return (false, $"HTTP {(int)response.StatusCode}");
            }

            if (!TryParseOrayResponse(raw, out var errorMessage))
            {
                _logService.Log(LogLevel.Warning, $"Oray API 返回失败: {errorMessage}");
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

    private bool TryParseOrayResponse(string raw, out string errorMessage)
    {
        errorMessage = string.Empty;

        try
        {
            var node = JsonNode.Parse(raw);
            var code = node?["code"]?.GetValue<int>();
            if (code != 200)
            {
                errorMessage = node?["message"]?.GetValue<string>() ?? "API 调用失败";
                return false;
            }

            return true;
        }
        catch (Exception ex)
        {
            errorMessage = $"解析响应失败: {ex.Message}";
            return false;
        }
    }
}