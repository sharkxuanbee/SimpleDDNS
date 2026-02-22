using SimpleDDNS.Core.Models;

namespace SimpleDDNS.Core.Abstractions;

public interface IDdnsProvider
{
    DdnsProviderType ProviderType { get; }

    Task<ProviderResult> UpdateRecordAsync(DdnsProfile profile, DnsRecordType recordType, string ipAddress, CancellationToken cancellationToken);

    Task<ProviderResult> TestAsync(DdnsProfile profile, CancellationToken cancellationToken);
}
