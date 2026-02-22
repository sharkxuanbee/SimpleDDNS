using SimpleDDNS.Core.Models;

namespace SimpleDDNS.Storage.Models;

internal sealed class StoredProfile
{
    public Guid Id { get; set; }

    public string Name { get; set; } = string.Empty;

    public bool IsEnabled { get; set; }

    public DdnsProviderType ProviderType { get; set; }

    public string Hostname { get; set; } = string.Empty;

    public bool EnableIPv4 { get; set; }

    public bool EnableIPv6 { get; set; }

    public int IntervalMinutes { get; set; }

    public CloudflareOptions Cloudflare { get; set; } = new();

    public GenericHttpOptions GenericHttp { get; set; } = new();

    public AliyunOptions Aliyun { get; set; } = new();

    public TencentCloudOptions TencentCloud { get; set; } = new();

    public OrayOptions Oray { get; set; } = new();

    public NoIPOptions NoIP { get; set; } = new();

    public DuckDnsOptions DuckDNS { get; set; } = new();

    public string EncryptedSecrets { get; set; } = string.Empty;
}
