using System.Net.Http;
using System.Text;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class NoIPProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;

    public NoIPProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://dynupdate.no-ip.com/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.NoIP;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPUsername))
        {
            return new ProviderResult(false, "No-IP Username 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPPassword))
        {
            return new ProviderResult(false, "No-IP Password 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.NoIP.Hostname))
        {
            return new ProviderResult(false, "No-IP Hostname 未填写");
        }

        var username = profile.Secrets.NoIPUsername;
        var password = profile.Secrets.NoIPPassword;
        var hostname = profile.NoIP.Hostname;

        var result = await UpdateDdnsRecordAsync(username, password, hostname, ipAddress, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return new ProviderResult(false, $"更新 DNS 记录失败: {result.Message}");
        }

        return new ProviderResult(true, $"已更新 {recordType} 记录 -> {ipAddress}");
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPUsername))
        {
            return new ProviderResult(false, "No-IP Username 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPPassword))
        {
            return new ProviderResult(false, "No-IP Password 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.NoIP.Hostname))
        {
            return new ProviderResult(false, "No-IP Hostname 未填写");
        }

        var username = profile.Secrets.NoIPUsername;
        var password = profile.Secrets.NoIPPassword;
        var hostname = profile.NoIP.Hostname;

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
        var requestUrl = $"https://dynupdate.no-ip.com/nic/update?hostname={Uri.EscapeDataString(hostname)}&myip={Uri.EscapeDataString(ipAddress)}";
        var credentials = Convert.ToBase64String(Encoding.ASCII.GetBytes($"{username}:{password}"));

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, requestUrl);
            request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Basic", credentials);
            request.Headers.UserAgent.Add(new System.Net.Http.Headers.ProductInfoHeaderValue("SimpleDDNS", "1.0"));
            using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
            var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return (false, $"HTTP {(int)response.StatusCode}: {raw}");
            }

            if (!TryParseNoIPResponse(raw, out var errorMessage))
            {
                _logService.Log(LogLevel.Warning, $"No-IP API 返回失败: {errorMessage}");
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

    private bool TryParseNoIPResponse(string raw, out string errorMessage)
    {
        errorMessage = string.Empty;

        try
        {
            var response = raw.Trim();
            if (response.StartsWith("good", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            else if (response.StartsWith("nochg", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            else if (response.StartsWith("nohost", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "主机名不存在";
                return false;
            }
            else if (response.StartsWith("badauth", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "认证失败";
                return false;
            }
            else if (response.StartsWith("badagent", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "客户端不被允许";
                return false;
            }
            else if (response.StartsWith("abuse", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "滥用检测";
                return false;
            }
            else if (response.StartsWith("911", StringComparison.OrdinalIgnoreCase))
            {
                errorMessage = "服务不可用";
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