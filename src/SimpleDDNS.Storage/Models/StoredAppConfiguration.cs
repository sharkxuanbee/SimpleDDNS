namespace SimpleDDNS.Storage.Models;

internal sealed class StoredAppConfiguration
{
    public List<StoredProfile> Profiles { get; set; } = new();

    public Core.Models.AppSettings Settings { get; set; } = new();
}
