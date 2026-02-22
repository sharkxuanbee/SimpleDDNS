using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Providers.Providers;

public sealed class AliyunProvider : IDdnsProvider, IDisposable
{
    private readonly HttpClient _httpClient;
    private readonly ILogService _logService;

    public AliyunProvider(ILogService logService)
    {
        _logService = logService;
        _httpClient = new HttpClient
        {
            BaseAddress = new Uri("https://alidns.aliyuncs.com/"),
            Timeout = TimeSpan.FromSeconds(10)
        };
    }

    public DdnsProviderType ProviderType => DdnsProviderType.Aliyun;

    public async Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeyId))
        {
            return new ProviderResult(false, "Aliyun AccessKeyId 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeySecret))
        {
            return new ProviderResult(false, "Aliyun AccessKeySecret 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Aliyun.DomainName))
        {
            return new ProviderResult(false, "Aliyun DomainName 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Hostname))
        {
            return new ProviderResult(false, "记录名未填写");
        }

        var accessKeyId = profile.Secrets.AliyunAccessKeyId;
        var accessKeySecret = profile.Secrets.AliyunAccessKeySecret;
        var domainName = profile.Aliyun.DomainName;
        var hostname = profile.Hostname;
        var rr = ExtractRR(hostname, domainName);
        var type = recordType.ToString();
        var ttl = NormalizeTtl(profile.Aliyun.Ttl);

        var existingRecord = await FindRecordAsync(accessKeyId, accessKeySecret, domainName, rr, type, cancellationToken).ConfigureAwait(false);
        if (!existingRecord.Success)
        {
            return new ProviderResult(false, existingRecord.Message);
        }

        if (!string.IsNullOrWhiteSpace(existingRecord.RecordId))
        {
            var updateResult = await UpdateRecordAsync(accessKeyId, accessKeySecret, existingRecord.RecordId, rr, type, ipAddress, ttl, cancellationToken).ConfigureAwait(false);
            if (!updateResult.Success)
            {
                return new ProviderResult(false, $"更新 DNS 记录失败: {updateResult.Message}");
            }
            return new ProviderResult(true, $"已更新 {type} 记录 -> {ipAddress}");
        }
        else
        {
            var addResult = await AddRecordAsync(accessKeyId, accessKeySecret, domainName, rr, type, ipAddress, ttl, cancellationToken).ConfigureAwait(false);
            if (!addResult.Success)
            {
                return new ProviderResult(false, $"创建 DNS 记录失败: {addResult.Message}");
            }
            return new ProviderResult(true, $"已创建 {type} 记录 -> {ipAddress}");
        }
    }

    public async Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeyId))
        {
            return new ProviderResult(false, "Aliyun AccessKeyId 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeySecret))
        {
            return new ProviderResult(false, "Aliyun AccessKeySecret 未填写");
        }

        if (string.IsNullOrWhiteSpace(profile.Aliyun.DomainName))
        {
            return new ProviderResult(false, "Aliyun DomainName 未填写");
        }

        var accessKeyId = profile.Secrets.AliyunAccessKeyId;
        var accessKeySecret = profile.Secrets.AliyunAccessKeySecret;
        var domainName = profile.Aliyun.DomainName;

        var testResult = await TestCredentialsAsync(accessKeyId, accessKeySecret, domainName, cancellationToken).ConfigureAwait(false);
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

    private async Task<(bool Success, string RecordId, string Message)> FindRecordAsync(string accessKeyId, string accessKeySecret, string domainName, string rr, string type, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "DescribeDomainRecords" },
            { "Version", "2015-01-09" },
            { "DomainName", domainName },
            { "RRKeyWord", rr },
            { "TypeKeyWord", type }
        };

        var requestUrl = BuildRequestUrl(parameters, accessKeyId, accessKeySecret);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        if (!result.Success)
        {
            return (false, string.Empty, result.Message);
        }

        var node = JsonNode.Parse(result.Raw);
        var records = node?["DomainRecords"]?["Record"]?.AsArray();
        if (records is not null && records.Count > 0)
        {
            var recordId = records[0]?["RecordId"]?.GetValue<string>();
            return (true, recordId ?? string.Empty, "ok");
        }

        return (true, string.Empty, "ok");
    }

    private async Task<(bool Success, string Message)> UpdateRecordAsync(string accessKeyId, string accessKeySecret, string recordId, string rr, string type, string value, int ttl, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "UpdateDomainRecord" },
            { "Version", "2015-01-09" },
            { "RecordId", recordId },
            { "RR", rr },
            { "Type", type },
            { "Value", value },
            { "TTL", ttl.ToString() }
        };

        var requestUrl = BuildRequestUrl(parameters, accessKeyId, accessKeySecret);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        return (result.Success, result.Message);
    }

    private async Task<(bool Success, string Message)> AddRecordAsync(string accessKeyId, string accessKeySecret, string domainName, string rr, string type, string value, int ttl, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "AddDomainRecord" },
            { "Version", "2015-01-09" },
            { "DomainName", domainName },
            { "RR", rr },
            { "Type", type },
            { "Value", value },
            { "TTL", ttl.ToString() }
        };

        var requestUrl = BuildRequestUrl(parameters, accessKeyId, accessKeySecret);
        var result = await SendRequestAsync(requestUrl, cancellationToken).ConfigureAwait(false);
        return (result.Success, result.Message);
    }

    private async Task<(bool Success, string Message)> TestCredentialsAsync(string accessKeyId, string accessKeySecret, string domainName, CancellationToken cancellationToken)
    {
        var parameters = new Dictionary<string, string>
        {
            { "Action", "DescribeDomainRecords" },
            { "Version", "2015-01-09" },
            { "DomainName", domainName },
            { "PageSize", "1" }
        };

        var requestUrl = BuildRequestUrl(parameters, accessKeyId, accessKeySecret);
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

            if (!TryParseAliyunResponse(raw, out var errorMessage))
            {
                _logService.Log(LogLevel.Warning, $"Aliyun API 返回失败: {errorMessage}");
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

    private string BuildRequestUrl(Dictionary<string, string> parameters, string accessKeyId, string accessKeySecret)
    {
        var sortedParameters = parameters.OrderBy(p => p.Key).ToDictionary(p => p.Key, p => p.Value);
        sortedParameters["AccessKeyId"] = accessKeyId;
        sortedParameters["Timestamp"] = DateTime.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'");
        sortedParameters["SignatureMethod"] = "HMAC-SHA1";
        sortedParameters["SignatureVersion"] = "1.0";
        sortedParameters["SignatureNonce"] = Guid.NewGuid().ToString();

        var canonicalizedQueryString = string.Join("&", sortedParameters.Select(p => $"{Uri.EscapeDataString(p.Key)}={Uri.EscapeDataString(p.Value)}"));
        var stringToSign = $"GET&{Uri.EscapeDataString("/")}&{Uri.EscapeDataString(canonicalizedQueryString)}";

        var signature = ComputeSignature(stringToSign, accessKeySecret);
        var requestUrl = $"https://alidns.aliyuncs.com/?{canonicalizedQueryString}&Signature={Uri.EscapeDataString(signature)}";

        return requestUrl;
    }

    private string ComputeSignature(string stringToSign, string accessKeySecret)
    {
        using var hmac = new HMACSHA1(Encoding.UTF8.GetBytes($"{accessKeySecret}&"));
        var hash = hmac.ComputeHash(Encoding.UTF8.GetBytes(stringToSign));
        return Convert.ToBase64String(hash);
    }

    private bool TryParseAliyunResponse(string raw, out string errorMessage)
    {
        errorMessage = string.Empty;

        try
        {
            var node = JsonNode.Parse(raw);
            var code = node?["Code"]?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(code))
            {
                errorMessage = node?["Message"]?.GetValue<string>() ?? code;
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

    private string ExtractRR(string hostname, string domainName)
    {
        if (hostname.Equals(domainName, StringComparison.OrdinalIgnoreCase))
        {
            return "@";
        }

        if (hostname.EndsWith($".{domainName}", StringComparison.OrdinalIgnoreCase))
        {
            return hostname[..^($".{domainName}".Length)];
        }

        return hostname;
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