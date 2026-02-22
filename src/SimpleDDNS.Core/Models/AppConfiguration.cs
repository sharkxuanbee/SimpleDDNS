namespace SimpleDDNS.Core.Models;

public sealed class AppConfiguration
{
    public List<DdnsProfile> Profiles { get; set; } = new();

    public AppSettings Settings { get; set; } = new();
}
