namespace SimpleDDNS.Core.Models;

public sealed class ProfileStatusChangedEventArgs(Guid profileId, ProfileRuntimeStatus runtimeStatus) : EventArgs
{
    public Guid ProfileId { get; } = profileId;

    public ProfileRuntimeStatus RuntimeStatus { get; } = runtimeStatus;
}
