namespace SimpleDDNS.Core.Models;

public sealed class CloudflareOptions
{
    public string ZoneName { get; set; } = string.Empty;

    public int Ttl { get; set; } = 1;

    public bool Proxied { get; set; }
}
