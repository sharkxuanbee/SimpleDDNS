namespace SimpleDDNS.Core.Models;

public sealed class ProfileSecrets
{
    public string CloudflareApiToken { get; set; } = string.Empty;

    public Dictionary<string, string> GenericHeaders { get; set; } = new(StringComparer.OrdinalIgnoreCase);

    public string AliyunAccessKeyId { get; set; } = string.Empty;

    public string AliyunAccessKeySecret { get; set; } = string.Empty;

    public string TencentCloudSecretId { get; set; } = string.Empty;

    public string TencentCloudSecretKey { get; set; } = string.Empty;

    public string OrayUsername { get; set; } = string.Empty;

    public string OrayPassword { get; set; } = string.Empty;

    public string NoIPUsername { get; set; } = string.Empty;

    public string NoIPPassword { get; set; } = string.Empty;

    public string DuckDnsToken { get; set; } = string.Empty;
}
