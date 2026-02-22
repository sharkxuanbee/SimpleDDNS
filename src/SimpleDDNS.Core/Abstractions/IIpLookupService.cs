using SimpleDDNS.Core.Models;

namespace SimpleDDNS.Core.Abstractions;

public interface IIpLookupService
{
    Task<IpLookupResult> LookupAsync(IpProtocol protocol, IReadOnlyList<string> endpoints, CancellationToken cancellationToken);
}
