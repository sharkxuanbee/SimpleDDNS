using System.Text.Json.Serialization;

namespace SimpleDDNS.Core.Models;

public sealed class DdnsProfile
{
    public Guid Id { get; set; } = Guid.NewGuid();

    public string Name { get; set; } = "新配置";

    public bool IsEnabled { get; set; } = true;

    public DdnsProviderType ProviderType { get; set; } = DdnsProviderType.Cloudflare;

    public string Hostname { get; set; } = string.Empty;

    public bool EnableIPv4 { get; set; } = true;

    public bool EnableIPv6 { get; set; }

    public int IntervalMinutes { get; set; } = 10;

    public CloudflareOptions Cloudflare { get; set; } = new();

    public GenericHttpOptions GenericHttp { get; set; } = new();

    public AliyunOptions Aliyun { get; set; } = new();

    public TencentCloudOptions TencentCloud { get; set; } = new();

    public OrayOptions Oray { get; set; } = new();

    public NoIPOptions NoIP { get; set; } = new();

    public DuckDnsOptions DuckDNS { get; set; } = new();

    [JsonIgnore]
    public ProfileSecrets Secrets { get; set; } = new();

    [JsonIgnore]
    public ProfileRuntimeStatus RuntimeStatus { get; set; } = new();
}
