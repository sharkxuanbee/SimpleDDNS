using SimpleDDNS.Core.Models;

namespace SimpleDDNS.Core.Abstractions;

public interface IAppConfigurationStore
{
    string ConfigurationPath { get; }

    Task<AppConfiguration> LoadAsync(CancellationToken cancellationToken = default);

    Task SaveAsync(AppConfiguration configuration, CancellationToken cancellationToken = default);

    Task ExportAsync(AppConfiguration configuration, string exportFilePath, bool includeSensitive, CancellationToken cancellationToken = default);
}
