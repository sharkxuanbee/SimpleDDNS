using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class TencentCloudProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;

    public TencentCloudProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://dnspod.tencentcloudapi.com/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.TencentCloud;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretId))
        {
            return new ProviderResult(false, "TencentCloud SecretId 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretKey))
        {
            return new ProviderResult(false, "TencentCloud SecretKey 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.TencentCloud.Domain))
        {
            return new ProviderResult(false, "TencentCloud Domain 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.TencentCloud.SubDomain))
        {
            return new ProviderResult(false, "TencentCloud SubDomain 未填写");
        }

        var secretId = profile.Secrets.TencentCloudSecretId;
        var secretKey = profile.Secrets.TencentCloudSecretKey;
        var domain = profile.TencentCloud.Domain;
        var subDomain = profile.TencentCloud.SubDomain;
        var type = recordType.ToString();
        var ttl = NormalizeTtl(profile.TencentCloud.Ttl);

        var domainInfo = await GetDomainInfoAsync(secretId, secretKey, domain, cancellationToken).ConfigureAwait(false);
        if (!domainInfo.Success)
        {
            return new ProviderResult(false, domainInfo.Message);
        }

        var domainId = domainInfo.DomainId;
        var existingRecord = await FindRecordAsync(secretId, secretKey, domainId, subDomain, type, cancellationToken).ConfigureAwait(false);
        if (!existingRecord.Success)
        {
            return new ProviderResult(false, existingRecord.Message);
        }

        if (!string.IsNullOrWhiteSpace(existingRecord.RecordId))
        {
            var updateResult = await UpdateRecordAsync(secretId, secretKey, domainId, existingRecord.RecordId, subDomain, type, ipAddress, ttl, cancellationToken).ConfigureAwait(false);
            if (!updateResult.Success)
            {
                return new ProviderResult(false, $"更新 DNS 记录失败: {updateResult.Message}");
            }
            return new ProviderResult(true, $"已更新 {type} 记录 -> {ipAddress}");
        }
        else
        {
            var addResult = await AddRecordAsync(secretId, secretKey, domainId, subDomain, type, ipAddress, ttl, cancellationToken).ConfigureAwait(false);
            if (!addResult.Success)
            {
                return new ProviderResult(false, $"创建 DNS 记录失败: {addResult.Message}");
            }
            return new ProviderResult(true, $"已创建 {type} 记录 -> {ipAddress}");
        }
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretId))
        {
            return new ProviderResult(false, "TencentCloud SecretId 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretKey))
        {
            return new ProviderResult(false, "TencentCloud SecretKey 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.TencentCloud.Domain))
        {
            return new ProviderResult(false, "TencentCloud Domain 未填写");
        }

        var secretId = profile.Secrets.TencentCloudSecretId;
        var secretKey = profile.Secrets.TencentCloudSecretKey;
        var domain = profile.TencentCloud.Domain;

        var testResult = await GetDomainInfoAsync(secretId, secretKey, domain, cancellationToken).ConfigureAwait(false);
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

    private async Task<(bool Success, string DomainId, string Message)> GetDomainInfoAsync(string secretId, string secretKey, string domain, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "DescribeDomain" },
            { "Version", "2021-03-23" },
            { "Domain", domain }
        };

        var requestUrl = BuildRequestUrl(parameters, secretId, secretKey);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return (false, string.Empty, result.Message);
        }

        var node = JsonNode.Parse(result.Raw);
        var domainId = node?["Response"]?["Domain"]?["Id"]?.GetValue<string>();
        if (string.IsNullOrWhiteSpace(domainId))
        {
            return (false, string.Empty, "获取域名信息失败");
        }

        return (true, domainId, "ok");
    }

    private async Task<(bool Success, string RecordId, string Message)> FindRecordAsync(string secretId, string secretKey, string domainId, string subDomain, string type, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "DescribeRecordList" },
            { "Version", "2021-03-23" },
            { "DomainId", domainId },
            { "Subdomain", subDomain },
            { "RecordType", type }
        };

        var requestUrl = BuildRequestUrl(parameters, secretId, secretKey);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return (false, string.Empty, result.Message);
        }

        var node = JsonNode.Parse(result.Raw);
        var records = node?["Response"]?["RecordList"]?.AsArray();
        if (records is not null && records.Count > 0)
        {
            var recordId = records[0]?["Id"]?.GetValue<string>();
            return (true, recordId ?? string.Empty, "ok");
        }

        return (true, string.Empty, "ok");
    }

    private async Task<(bool Success, string Message)> UpdateRecordAsync(string secretId, string secretKey, string domainId, string recordId, string subDomain, string type, string value, int ttl, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "ModifyRecord" },
            { "Version", "2021-03-23" },
            { "DomainId", domainId },
            { "RecordId", recordId },
            { "Subdomain", subDomain },
            { "RecordType", type },
            { "RecordLine", "默认" },
            { "Value", value },
            { "TTL", ttl.ToString() }
        };

        var requestUrl = BuildRequestUrl(parameters, secretId, secretKey);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        return (result.Success, result.Message);
    }

    private async Task<(bool Success, string Message)> AddRecordAsync(string secretId, string secretKey, string domainId, string subDomain, string type, string value, int ttl, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "CreateRecord" },
            { "Version", "2021-03-23" },
            { "DomainId", domainId },
            { "Subdomain", subDomain },
            { "RecordType", type },
            { "RecordLine", "默认" },
            { "Value", value },
            { "TTL", ttl.ToString() }
        };

        var requestUrl = BuildRequestUrl(parameters, secretId, secretKey);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        return (result.Success, result.Message);
    }

    private async Task<(bool Success, string Message, string Raw)> SendRequestAsync(string requestUrl, CancellationToken cancellationToken)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, requestUrl);
            using var response = await _httpClient.SendAsync(request, cancellationToken).ConfigureAwait(false);
            var raw = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return (false, $"HTTP {(int)response.StatusCode}", raw);
            }

            if (!TryParseTencentCloudResponse(raw, out var errorMessage))
            {
                _logService.Log(LogLevel.Warning, $"TencentCloud API 返回失败: {errorMessage}");
                return (false, errorMessage, raw);
            }

            return (true, "ok", raw);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return (false, "请求超时", string.Empty);
        }
        catch (HttpRequestException ex)
        {
            return (false, $"网络请求失败: {ex.Message}", string.Empty);
        }
        catch (Exception ex)
        {
            return (false, $"请求失败: {ex.Message}", string.Empty);
        }
    }

    private string BuildRequestUrl(Dictionary<string, string> parameters, string secretId, string secretKey)
    {
        var sortedParameters = parameters.OrderBy(p => p.Key).ToDictionary(p => p.Key, p => p.Value);
        sortedParameters["SecretId"] = secretId;
        sortedParameters["Timestamp"] = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
        sortedParameters["Nonce"] = new Random().Next(10000, 99999).ToString();
        sortedParameters["Region"] = "ap-guangzhou";

        var canonicalizedQueryString = string.Join("&", sortedParameters.Select(p => $"{Uri.EscapeDataString(p.Key)}={Uri.EscapeDataString(p.Value)}"));
        var stringToSign = $"GET&{Uri.EscapeDataString("/")}&{Uri.EscapeDataString(canonicalizedQueryString)}";

        var signature = ComputeSignature(stringToSign, secretKey);
        var requestUrl = $"https://dnspod.tencentcloudapi.com/?{canonicalizedQueryString}&Signature={Uri.EscapeDataString(signature)}";

        return requestUrl;
    }

    private string ComputeSignature(string stringToSign, string secretKey)
    {
        using var hmac = new HMACSHA1(Encoding.UTF8.GetBytes(secretKey));
        var hash = hmac.ComputeHash(Encoding.UTF8.GetBytes(stringToSign));
        return Convert.ToBase64String(hash);
    }

    private bool TryParseTencentCloudResponse(string raw, out string errorMessage)
    {
        errorMessage = string.Empty;

        try
        {
            var node = JsonNode.Parse(raw);
            var error = node?["Response"]?["Error"];
            if (error is not null)
            {
                errorMessage = error["Message"]?.GetValue<string>() ?? "API 调用失败";
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

    private int NormalizeTtl(int ttl)
    {
        if (ttl <= 0)
        {
            return 600;
        }

        return Math.Clamp(ttl, 60, 86400);
    }
}