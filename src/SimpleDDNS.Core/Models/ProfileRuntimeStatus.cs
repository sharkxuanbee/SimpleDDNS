using System.Text.Json.Serialization;

namespace SimpleDDNS.Core.Models;

public sealed class ProfileRuntimeStatus
{
    public DateTimeOffset? LastUpdatedAt { get; set; }

    public string CurrentIPv4 { get; set; } = string.Empty;

    public string CurrentIPv6 { get; set; } = string.Empty;

    public string LastResult { get; set; } = "尚未执行";

    public bool IsRunning { get; set; }

    [JsonIgnore]
    public SemaphoreSlim RunGate { get; } = new(1, 1);
}
