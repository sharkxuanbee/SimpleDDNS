using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;
using SimpleDDNS.Storage;

namespace SimpleDDNS.Tests;

public class ConfigurationStoreTests
{
    [Fact]
    public async Task SaveAndLoad_ShouldEncryptAndDecryptSecrets()
    {
        var tempDirectory = Path.Combine(Path.GetTempPath(), "SimpleDDNS.Tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(tempDirectory);
        var configPath = Path.Combine(tempDirectory, "config.json");

        try
        {
            var log = new InMemoryLogService();
            var store = new JsonConfigurationStore(log, configPath);

            var source = new AppConfiguration
            {
                Settings = new AppSettings(),
                Profiles =
                [
                    new DdnsProfile
                    {
                        Name = "CF",
                        Hostname = "home.example.com",
                        ProviderType = DdnsProviderType.Cloudflare,
                        EnableIPv4 = true,
                        EnableIPv6 = true,
                        IntervalMinutes = 10,
                        Cloudflare = new CloudflareOptions
                        {
                            ZoneName = "example.com",
                            Ttl = 1,
                            Proxied = true
                        },
                        Secrets = new ProfileSecrets
                        {
                            CloudflareApiToken = "token-secret-123",
                            GenericHeaders = new Dictionary<string, string>
                            {
                                ["Authorization"] = "Bearer abc"
                            }
                        }
                    }
                ]
            };

            await store.SaveAsync(source);

            var raw = await File.ReadAllTextAsync(configPath);
            Assert.DoesNotContain("token-secret-123", raw);
            Assert.DoesNotContain("Bearer abc", raw);

            var loaded = await store.LoadAsync();
            Assert.Single(loaded.Profiles);
            Assert.Equal("token-secret-123", loaded.Profiles[0].Secrets.CloudflareApiToken);
            Assert.Equal("Bearer abc", loaded.Profiles[0].Secrets.GenericHeaders["Authorization"]);
        }
        finally
        {
            if (Directory.Exists(tempDirectory))
            {
                Directory.Delete(tempDirectory, recursive: true);
            }
        }
    }
}
