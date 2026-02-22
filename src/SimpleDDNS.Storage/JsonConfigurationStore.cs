using System.Text.Json;
using System.Text.Json.Serialization;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;
using SimpleDDNS.Storage.Models;
using SimpleDDNS.Storage.Security;

namespace SimpleDDNS.Storage;

public sealed class JsonConfigurationStore : IAppConfigurationStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    private readonly DpapiSecretProtector _secretProtector;
    private readonly ILogService _logService;

    public JsonConfigurationStore(ILogService logService, string? configurationPath = null)
    {
        _logService = logService;
        ConfigurationPath = configurationPath ?? GetDefaultPath();
        _secretProtector = new DpapiSecretProtector(GetEntropyPath(ConfigurationPath));
    }

    public string ConfigurationPath { get; }

    public async Task<AppConfiguration> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(ConfigurationPath))
        {
            var initial = new AppConfiguration();
            await SaveAsync(initial, cancellationToken).ConfigureAwait(false);
            return initial;
        }

        return await LoadFromPathAsync(ConfigurationPath, cancellationToken).ConfigureAwait(false);
    }

    public async Task<AppConfiguration> LoadFromPathAsync(string path, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(path))
        {
            return new AppConfiguration();
        }

        var json = await File.ReadAllTextAsync(path, cancellationToken).ConfigureAwait(false);
        if (string.IsNullOrWhiteSpace(json))
        {
            return new AppConfiguration();
        }

        var stored = JsonSerializer.Deserialize<StoredAppConfiguration>(json, JsonOptions) ?? new StoredAppConfiguration();
        var configuration = new AppConfiguration
        {
            Settings = stored.Settings ?? new AppSettings(),
            Profiles = stored.Profiles.Select(MapToRuntimeProfile).ToList()
        };

        foreach (var profile in configuration.Profiles)
        {
            if (profile.IntervalMinutes <= 0)
            {
                profile.IntervalMinutes = configuration.Settings.DefaultIntervalMinutes;
            }
        }

        return configuration;
    }

    public async Task SaveAsync(AppConfiguration configuration, CancellationToken cancellationToken = default)
    {
        var stored = MapToStored(configuration, includeSensitive: true);
        await PersistAsync(stored, ConfigurationPath, cancellationToken).ConfigureAwait(false);
    }

    public async Task ExportAsync(AppConfiguration configuration, string exportFilePath, bool includeSensitive, CancellationToken cancellationToken = default)
    {
        var stored = MapToStored(configuration, includeSensitive);
        await PersistAsync(stored, exportFilePath, cancellationToken).ConfigureAwait(false);
    }

    private static string GetDefaultPath()
    {
        var baseDir = AppContext.BaseDirectory;
        var localDataDir = Path.Combine(baseDir, "data");
        var localConfig = Path.Combine(localDataDir, "config.json");
        var appDataDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SimpleDDNS");
        var appDataConfig = Path.Combine(appDataDir, "config.json");
        var appDataEntropy = Path.Combine(appDataDir, "entropy.bin");
        var localEntropy = Path.Combine(localDataDir, "entropy.bin");

        try
        {
            Directory.CreateDirectory(localDataDir);

            if (!File.Exists(localConfig) && File.Exists(appDataConfig))
            {
                File.Copy(appDataConfig, localConfig, overwrite: false);
            }

            if (!File.Exists(localEntropy) && File.Exists(appDataEntropy))
            {
                File.Copy(appDataEntropy, localEntropy, overwrite: false);
            }

            return localConfig;
        }
        catch
        {
            return appDataConfig;
        }
    }
    
    private static string GetEntropyPath(string configurationPath)
    {
        var directory = Path.GetDirectoryName(configurationPath);
        if (string.IsNullOrWhiteSpace(directory))
        {
            directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SimpleDDNS");
        }

        return Path.Combine(directory, "entropy.bin");
    }

    private async Task PersistAsync(StoredAppConfiguration configuration, string path, CancellationToken cancellationToken)
    {
        var directory = Path.GetDirectoryName(path);
        if (!string.IsNullOrWhiteSpace(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var json = JsonSerializer.Serialize(configuration, JsonOptions);
        await File.WriteAllTextAsync(path, json, cancellationToken).ConfigureAwait(false);
    }

    private StoredAppConfiguration MapToStored(AppConfiguration runtime, bool includeSensitive)
    {
        var storedProfiles = runtime.Profiles.Select(profile =>
        {
            var secretJson = JsonSerializer.Serialize(profile.Secrets, JsonOptions);
            var encrypted = includeSensitive ? _secretProtector.Protect(secretJson) : string.Empty;

            return new StoredProfile
            {
                Id = profile.Id,
                Name = profile.Name,
                IsEnabled = profile.IsEnabled,
                ProviderType = profile.ProviderType,
                Hostname = profile.Hostname,
                EnableIPv4 = profile.EnableIPv4,
                EnableIPv6 = profile.EnableIPv6,
                IntervalMinutes = profile.IntervalMinutes,
                Cloudflare = profile.Cloudflare,
                GenericHttp = profile.GenericHttp,
                Aliyun = profile.Aliyun,
                TencentCloud = profile.TencentCloud,
                Oray = profile.Oray,
                NoIP = profile.NoIP,
                DuckDNS = profile.DuckDNS,
                EncryptedSecrets = encrypted
            };
        }).ToList();

        return new StoredAppConfiguration
        {
            Profiles = storedProfiles,
            Settings = runtime.Settings
        };
    }

    private DdnsProfile MapToRuntimeProfile(StoredProfile stored)
    {
        var profile = new DdnsProfile
        {
            Id = stored.Id == Guid.Empty ? Guid.NewGuid() : stored.Id,
            Name = stored.Name,
            IsEnabled = stored.IsEnabled,
            ProviderType = stored.ProviderType,
            Hostname = stored.Hostname,
            EnableIPv4 = stored.EnableIPv4,
            EnableIPv6 = stored.EnableIPv6,
            IntervalMinutes = stored.IntervalMinutes,
            Cloudflare = stored.Cloudflare ?? new CloudflareOptions(),
            GenericHttp = stored.GenericHttp ?? new GenericHttpOptions(),
            Aliyun = stored.Aliyun ?? new AliyunOptions(),
            TencentCloud = stored.TencentCloud ?? new TencentCloudOptions(),
            Oray = stored.Oray ?? new OrayOptions(),
            NoIP = stored.NoIP ?? new NoIPOptions(),
            DuckDNS = stored.DuckDNS ?? new DuckDnsOptions(),
            RuntimeStatus = new ProfileRuntimeStatus()
        };

        if (string.IsNullOrWhiteSpace(stored.EncryptedSecrets))
        {
            profile.Secrets = new ProfileSecrets();
            return profile;
        }

        try
        {
            var plainJson = _secretProtector.Unprotect(stored.EncryptedSecrets);
            profile.Secrets = JsonSerializer.Deserialize<ProfileSecrets>(plainJson, JsonOptions) ?? new ProfileSecrets();
        }
        catch (Exception ex)
        {
            profile.Secrets = new ProfileSecrets();
            _logService.Log(LogLevel.Warning, $"解密 Profile({stored.Name}) 敏感信息失败，已使用空凭据", stored.Name, ex);
        }

        return profile;
    }
}
