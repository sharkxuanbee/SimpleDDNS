namespace SimpleDDNS.Core.Models;

public sealed class AppSettings
{
    public bool StartWithWindows { get; set; }

    public bool MinimizeToTrayOnClose { get; set; } = true;

    public int DefaultIntervalMinutes { get; set; } = 10;

    public List<string> IPv4ProbeEndpoints { get; set; } =
    [
        "https://api.ipify.org",
        "https://ipv4.icanhazip.com",
        "https://v4.ident.me"
    ];

    public List<string> IPv6ProbeEndpoints { get; set; } =
    [
        "https://api6.ipify.org",
        "https://ipv6.icanhazip.com",
        "https://v6.ident.me"
    ];
}
