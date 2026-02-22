using System.Text;
using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Controls;
using SimpleDDNS.Core.Models;
using MessageBox = System.Windows.MessageBox;

namespace SimpleDDNS.App.Windows;

public partial class ProfileWizardWindow : Window
{
    private static readonly Regex HostnameRegex = new("^[a-zA-Z0-9.-]+$", RegexOptions.Compiled);
    private readonly Guid _profileId;
    private readonly Func<DdnsProfile, CancellationToken, Task<ProviderResult>>? _testCallback;
    private int _stepIndex;

    public ProfileWizardWindow(
        DdnsProfile? existing,
        int defaultIntervalMinutes,
        Func<DdnsProfile, CancellationToken, Task<ProviderResult>>? testCallback)
    {
        InitializeComponent();

        _testCallback = testCallback;
        var seed = existing is null ? CreateDefault(defaultIntervalMinutes) : Clone(existing);
        _profileId = seed.Id;

        LoadProfile(seed);
        StepTabs.SelectedIndex = 0;
        _stepIndex = 0;
        UpdateProviderPanels();
        UpdateButtons();
    }

    public DdnsProfile ResultProfile { get; private set; } = new();

    private void ProviderComboBox_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        UpdateProviderPanels();
        UpdateSummary();
    }

    private void BackButton_OnClick(object sender, RoutedEventArgs e)
    {
        if (_stepIndex <= 0)
        {
            return;
        }

        _stepIndex--;
        StepTabs.SelectedIndex = _stepIndex;
        UpdateButtons();
    }

    private void NextButton_OnClick(object sender, RoutedEventArgs e)
    {
        if (_stepIndex >= StepTabs.Items.Count - 1)
        {
            return;
        }

        _stepIndex++;
        StepTabs.SelectedIndex = _stepIndex;

        if (_stepIndex == StepTabs.Items.Count - 1)
        {
            UpdateSummary();
        }

        UpdateButtons();
    }

    private void FinishButton_OnClick(object sender, RoutedEventArgs e)
    {
        if (!TryBuildProfile(strictValidation: true, out var profile, out var error))
        {
            MessageBox.Show(this, error, "校验失败", MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        ResultProfile = profile;
        DialogResult = true;
    }

    private void CancelButton_OnClick(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }

    private async void TestRequestButton_OnClick(object sender, RoutedEventArgs e)
    {
        if (_testCallback is null)
        {
            MessageBox.Show(this, "当前不可用测试请求", "提示", MessageBoxButton.OK, MessageBoxImage.Information);
            return;
        }

        if (!TryBuildProfile(strictValidation: false, out var profile, out var error))
        {
            MessageBox.Show(this, error, "校验失败", MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        try
        {
            TestRequestButton.IsEnabled = false;
            TestRequestButton.Content = "测试中...";

            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            var result = await _testCallback(profile, cts.Token).ConfigureAwait(true);
            var title = result.Success ? "测试成功" : "测试失败";
            var icon = result.Success ? MessageBoxImage.Information : MessageBoxImage.Error;
            var message = string.IsNullOrWhiteSpace(result.RawResponse)
                ? result.Message
                : $"{result.Message}{Environment.NewLine}{Environment.NewLine}{Trim(result.RawResponse, 600)}";
            MessageBox.Show(this, message, title, MessageBoxButton.OK, icon);
        }
        finally
        {
            TestRequestButton.IsEnabled = true;
            TestRequestButton.Content = "测试请求";
        }
    }

    private void LoadProfile(DdnsProfile profile)
    {
        ProfileNameTextBox.Text = profile.Name;
        HostnameTextBox.Text = profile.Hostname;
        ZoneNameTextBox.Text = profile.Cloudflare.ZoneName;
        CloudflareTokenPasswordBox.Password = profile.Secrets.CloudflareApiToken;
        GenericHeadersTextBox.Text = string.Join(Environment.NewLine, profile.Secrets.GenericHeaders.Select(x => $"{x.Key}: {x.Value}"));

        AliyunDomainNameTextBox.Text = profile.Aliyun.DomainName;
        AliyunAccessKeyIdPasswordBox.Password = profile.Secrets.AliyunAccessKeyId;
        AliyunAccessKeySecretPasswordBox.Password = profile.Secrets.AliyunAccessKeySecret;

        TencentCloudDomainTextBox.Text = profile.TencentCloud.Domain;
        TencentCloudSubDomainTextBox.Text = profile.TencentCloud.SubDomain;
        TencentCloudSecretIdPasswordBox.Password = profile.Secrets.TencentCloudSecretId;
        TencentCloudSecretKeyPasswordBox.Password = profile.Secrets.TencentCloudSecretKey;

        OrayDomainTextBox.Text = profile.Oray.Domain;
        OraySubDomainTextBox.Text = profile.Oray.SubDomain;
        OrayUsernameTextBox.Text = profile.Secrets.OrayUsername;
        OrayPasswordPasswordBox.Password = profile.Secrets.OrayPassword;

        NoIPHostnameTextBox.Text = profile.NoIP.Hostname;
        NoIPUsernameTextBox.Text = profile.Secrets.NoIPUsername;
        NoIPPasswordPasswordBox.Password = profile.Secrets.NoIPPassword;

        DuckDNSDomainTextBox.Text = profile.DuckDNS.Domain;
        DuckDNSTokenPasswordBox.Password = profile.Secrets.DuckDnsToken;

        EnableIPv4CheckBox.IsChecked = profile.EnableIPv4;
        EnableIPv6CheckBox.IsChecked = profile.EnableIPv6;
        IntervalMinutesTextBox.Text = profile.IntervalMinutes.ToString();

        CloudflareTtlTextBox.Text = profile.Cloudflare.Ttl.ToString();
        CloudflareProxiedCheckBox.IsChecked = profile.Cloudflare.Proxied;

        GenericUrlTemplateTextBox.Text = profile.GenericHttp.UrlTemplate;
        GenericBodyTemplateTextBox.Text = profile.GenericHttp.JsonBodyTemplate;

        SelectProvider(profile.ProviderType);
        SelectMethod(profile.GenericHttp.Method);
    }

    private bool TryBuildProfile(bool strictValidation, out DdnsProfile profile, out string error)
    {
        error = string.Empty;
        profile = new DdnsProfile
        {
            Id = _profileId,
            Name = ProfileNameTextBox.Text.Trim(),
            ProviderType = GetSelectedProvider(),
            Hostname = HostnameTextBox.Text.Trim(),
            EnableIPv4 = EnableIPv4CheckBox.IsChecked == true,
            EnableIPv6 = EnableIPv6CheckBox.IsChecked == true,
            IsEnabled = true,
            Cloudflare = new CloudflareOptions
            {
                ZoneName = ZoneNameTextBox.Text.Trim(),
                Ttl = ParseInt(CloudflareTtlTextBox.Text, 1),
                Proxied = CloudflareProxiedCheckBox.IsChecked == true
            },
            Aliyun = new AliyunOptions
            {
                DomainName = AliyunDomainNameTextBox.Text.Trim(),
                Ttl = ParseInt(CloudflareTtlTextBox.Text, 600)
            },
            TencentCloud = new TencentCloudOptions
            {
                Domain = TencentCloudDomainTextBox.Text.Trim(),
                SubDomain = TencentCloudSubDomainTextBox.Text.Trim(),
                Ttl = ParseInt(CloudflareTtlTextBox.Text, 600)
            },
            Oray = new OrayOptions
            {
                Domain = OrayDomainTextBox.Text.Trim(),
                SubDomain = OraySubDomainTextBox.Text.Trim()
            },
            NoIP = new NoIPOptions
            {
                Hostname = NoIPHostnameTextBox.Text.Trim()
            },
            DuckDNS = new DuckDnsOptions
            {
                Domain = DuckDNSDomainTextBox.Text.Trim()
            },
            GenericHttp = new GenericHttpOptions
            {
                UrlTemplate = GenericUrlTemplateTextBox.Text.Trim(),
                Method = GetSelectedMethod(),
                JsonBodyTemplate = GenericBodyTemplateTextBox.Text
            },
            Secrets = new ProfileSecrets
            {
                CloudflareApiToken = CloudflareTokenPasswordBox.Password,
                AliyunAccessKeyId = AliyunAccessKeyIdPasswordBox.Password,
                AliyunAccessKeySecret = AliyunAccessKeySecretPasswordBox.Password,
                TencentCloudSecretId = TencentCloudSecretIdPasswordBox.Password,
                TencentCloudSecretKey = TencentCloudSecretKeyPasswordBox.Password,
                OrayUsername = OrayUsernameTextBox.Text.Trim(),
                OrayPassword = OrayPasswordPasswordBox.Password,
                NoIPUsername = NoIPUsernameTextBox.Text.Trim(),
                NoIPPassword = NoIPPasswordPasswordBox.Password,
                DuckDnsToken = DuckDNSTokenPasswordBox.Password,
                GenericHeaders = ParseHeaders(GenericHeadersTextBox.Text)
            },
            RuntimeStatus = new ProfileRuntimeStatus()
        };

        profile.IntervalMinutes = ParseInt(IntervalMinutesTextBox.Text, 10);

        if (!strictValidation)
        {
            return true;
        }

        if (string.IsNullOrWhiteSpace(profile.Name))
        {
            error = "配置名称不能为空";
            return false;
        }

        if (string.IsNullOrWhiteSpace(profile.Hostname))
        {
            error = "记录名不能为空";
            return false;
        }

        if (!HostnameRegex.IsMatch(profile.Hostname))
        {
            error = "记录名包含非法字符，仅允许字母、数字、点和横线";
            return false;
        }

        if (!profile.EnableIPv4 && !profile.EnableIPv6)
        {
            error = "IPv4/IPv6 至少启用一个";
            return false;
        }

        if (profile.IntervalMinutes <= 0)
        {
            error = "轮询间隔必须大于 0";
            return false;
        }

        if (profile.ProviderType == DdnsProviderType.Cloudflare)
        {
            if (string.IsNullOrWhiteSpace(profile.Cloudflare.ZoneName))
            {
                error = "Cloudflare ZoneName 不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.CloudflareApiToken))
            {
                error = "Cloudflare API Token 不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.Aliyun)
        {
            if (string.IsNullOrWhiteSpace(profile.Aliyun.DomainName))
            {
                error = "阿里云域名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeyId))
            {
                error = "阿里云 AccessKeyId 不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeySecret))
            {
                error = "阿里云 AccessKeySecret 不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.TencentCloud)
        {
            if (string.IsNullOrWhiteSpace(profile.TencentCloud.Domain))
            {
                error = "腾讯云域名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretId))
            {
                error = "腾讯云 SecretId 不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretKey))
            {
                error = "腾讯云 SecretKey 不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.Oray)
        {
            if (string.IsNullOrWhiteSpace(profile.Oray.Domain))
            {
                error = "花生壳域名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.OrayUsername))
            {
                error = "花生壳用户名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.OrayPassword))
            {
                error = "花生壳密码不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.NoIP)
        {
            if (string.IsNullOrWhiteSpace(profile.NoIP.Hostname))
            {
                error = "No-IP 主机名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPUsername))
            {
                error = "No-IP 用户名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.NoIPPassword))
            {
                error = "No-IP 密码不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.DuckDNS)
        {
            if (string.IsNullOrWhiteSpace(profile.DuckDNS.Domain))
            {
                error = "DuckDNS 域名不能为空";
                return false;
            }

            if (string.IsNullOrWhiteSpace(profile.Secrets.DuckDnsToken))
            {
                error = "DuckDNS Token 不能为空";
                return false;
            }
        }

        if (profile.ProviderType == DdnsProviderType.GenericHttp)
        {
            if (string.IsNullOrWhiteSpace(profile.GenericHttp.UrlTemplate))
            {
                error = "通用 HTTP 的 URL 模板不能为空";
                return false;
            }

            if (!profile.GenericHttp.UrlTemplate.StartsWith("http://", StringComparison.OrdinalIgnoreCase) &&
                !profile.GenericHttp.UrlTemplate.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
            {
                error = "通用 HTTP 必须使用 http:// 或 https://";
                return false;
            }
        }

        return true;
    }

    private void UpdateSummary()
    {
        if (!TryBuildProfile(strictValidation: false, out var profile, out _))
        {
            SummaryTextBox.Text = "无法生成预览";
            return;
        }

        var builder = new StringBuilder();
        builder.AppendLine($"名称: {profile.Name}");
        builder.AppendLine($"Provider: {profile.ProviderType}");
        builder.AppendLine($"记录名: {profile.Hostname}");
        builder.AppendLine($"启用 IPv4: {profile.EnableIPv4}");
        builder.AppendLine($"启用 IPv6: {profile.EnableIPv6}");
        builder.AppendLine($"轮询间隔: {profile.IntervalMinutes} 分钟");

        if (profile.ProviderType == DdnsProviderType.Cloudflare)
        {
            builder.AppendLine($"ZoneName: {profile.Cloudflare.ZoneName}");
            builder.AppendLine($"TTL: {profile.Cloudflare.Ttl}");
            builder.AppendLine($"Proxied: {profile.Cloudflare.Proxied}");
            builder.AppendLine($"Token: {(string.IsNullOrWhiteSpace(profile.Secrets.CloudflareApiToken) ? "未填写" : "已填写")}");
        }
        else if (profile.ProviderType == DdnsProviderType.Aliyun)
        {
            builder.AppendLine($"域名: {profile.Aliyun.DomainName}");
            builder.AppendLine($"TTL: {profile.Aliyun.Ttl}");
            builder.AppendLine($"AccessKeyId: {(string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeyId) ? "未填写" : "已填写")}");
            builder.AppendLine($"AccessKeySecret: {(string.IsNullOrWhiteSpace(profile.Secrets.AliyunAccessKeySecret) ? "未填写" : "已填写")}");
        }
        else if (profile.ProviderType == DdnsProviderType.TencentCloud)
        {
            builder.AppendLine($"域名: {profile.TencentCloud.Domain}");
            builder.AppendLine($"子域名: {profile.TencentCloud.SubDomain}");
            builder.AppendLine($"TTL: {profile.TencentCloud.Ttl}");
            builder.AppendLine($"SecretId: {(string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretId) ? "未填写" : "已填写")}");
            builder.AppendLine($"SecretKey: {(string.IsNullOrWhiteSpace(profile.Secrets.TencentCloudSecretKey) ? "未填写" : "已填写")}");
        }
        else if (profile.ProviderType == DdnsProviderType.Oray)
        {
            builder.AppendLine($"域名: {profile.Oray.Domain}");
            builder.AppendLine($"子域名: {profile.Oray.SubDomain}");
            builder.AppendLine($"用户名: {(string.IsNullOrWhiteSpace(profile.Secrets.OrayUsername) ? "未填写" : "已填写")}");
            builder.AppendLine($"密码: {(string.IsNullOrWhiteSpace(profile.Secrets.OrayPassword) ? "未填写" : "已填写")}");
        }
        else if (profile.ProviderType == DdnsProviderType.NoIP)
        {
            builder.AppendLine($"主机名: {profile.NoIP.Hostname}");
            builder.AppendLine($"用户名: {(string.IsNullOrWhiteSpace(profile.Secrets.NoIPUsername) ? "未填写" : "已填写")}");
            builder.AppendLine($"密码: {(string.IsNullOrWhiteSpace(profile.Secrets.NoIPPassword) ? "未填写" : "已填写")}");
        }
        else if (profile.ProviderType == DdnsProviderType.DuckDNS)
        {
            builder.AppendLine($"域名: {profile.DuckDNS.Domain}");
            builder.AppendLine($"Token: {(string.IsNullOrWhiteSpace(profile.Secrets.DuckDnsToken) ? "未填写" : "已填写")}");
        }
        else
        {
            builder.AppendLine($"URL 模板: {profile.GenericHttp.UrlTemplate}");
            builder.AppendLine($"方法: {profile.GenericHttp.Method}");
            builder.AppendLine($"Header 数量: {profile.Secrets.GenericHeaders.Count}");
        }

        SummaryTextBox.Text = builder.ToString();
    }

    private void UpdateProviderPanels()
    {
        var providerType = GetSelectedProvider();
        
        CloudflareDomainPanel.Visibility = providerType == DdnsProviderType.Cloudflare ? Visibility.Visible : Visibility.Collapsed;
        AliyunDomainPanel.Visibility = providerType == DdnsProviderType.Aliyun ? Visibility.Visible : Visibility.Collapsed;
        TencentCloudDomainPanel.Visibility = providerType == DdnsProviderType.TencentCloud ? Visibility.Visible : Visibility.Collapsed;
        OrayDomainPanel.Visibility = providerType == DdnsProviderType.Oray ? Visibility.Visible : Visibility.Collapsed;
        NoIPDomainPanel.Visibility = providerType == DdnsProviderType.NoIP ? Visibility.Visible : Visibility.Collapsed;
        DuckDNSDomainPanel.Visibility = providerType == DdnsProviderType.DuckDNS ? Visibility.Visible : Visibility.Collapsed;
        
        CloudflareCredentialPanel.Visibility = providerType == DdnsProviderType.Cloudflare ? Visibility.Visible : Visibility.Collapsed;
        AliyunCredentialPanel.Visibility = providerType == DdnsProviderType.Aliyun ? Visibility.Visible : Visibility.Collapsed;
        TencentCloudCredentialPanel.Visibility = providerType == DdnsProviderType.TencentCloud ? Visibility.Visible : Visibility.Collapsed;
        OrayCredentialPanel.Visibility = providerType == DdnsProviderType.Oray ? Visibility.Visible : Visibility.Collapsed;
        NoIPCredentialPanel.Visibility = providerType == DdnsProviderType.NoIP ? Visibility.Visible : Visibility.Collapsed;
        DuckDNSCredentialPanel.Visibility = providerType == DdnsProviderType.DuckDNS ? Visibility.Visible : Visibility.Collapsed;
        GenericCredentialPanel.Visibility = providerType == DdnsProviderType.GenericHttp ? Visibility.Visible : Visibility.Collapsed;
        
        CloudflareAdvancedPanel.Visibility = providerType == DdnsProviderType.Cloudflare ? Visibility.Visible : Visibility.Collapsed;
        GenericAdvancedPanel.Visibility = providerType == DdnsProviderType.GenericHttp ? Visibility.Visible : Visibility.Collapsed;
    }

    private void UpdateButtons()
    {
        BackButton.IsEnabled = _stepIndex > 0;
        NextButton.IsEnabled = _stepIndex < StepTabs.Items.Count - 1;
    }

    private void SelectProvider(DdnsProviderType providerType)
    {
        switch (providerType)
        {
            case DdnsProviderType.Cloudflare:
                ProviderComboBox.SelectedIndex = 0;
                break;
            case DdnsProviderType.GenericHttp:
                ProviderComboBox.SelectedIndex = 1;
                break;
            case DdnsProviderType.Aliyun:
                ProviderComboBox.SelectedIndex = 2;
                break;
            case DdnsProviderType.TencentCloud:
                ProviderComboBox.SelectedIndex = 3;
                break;
            case DdnsProviderType.Oray:
                ProviderComboBox.SelectedIndex = 4;
                break;
            case DdnsProviderType.NoIP:
                ProviderComboBox.SelectedIndex = 5;
                break;
            case DdnsProviderType.DuckDNS:
                ProviderComboBox.SelectedIndex = 6;
                break;
        }
    }

    private void SelectMethod(HttpUpdateMethod method)
    {
        GenericMethodComboBox.SelectedIndex = method == HttpUpdateMethod.Post ? 1 : 0;
    }

    private DdnsProviderType GetSelectedProvider()
    {
        switch (ProviderComboBox.SelectedIndex)
        {
            case 0:
                return DdnsProviderType.Cloudflare;
            case 1:
                return DdnsProviderType.GenericHttp;
            case 2:
                return DdnsProviderType.Aliyun;
            case 3:
                return DdnsProviderType.TencentCloud;
            case 4:
                return DdnsProviderType.Oray;
            case 5:
                return DdnsProviderType.NoIP;
            case 6:
                return DdnsProviderType.DuckDNS;
            default:
                return DdnsProviderType.Cloudflare;
        }
    }

    private HttpUpdateMethod GetSelectedMethod()
    {
        return GenericMethodComboBox.SelectedIndex == 1 ? HttpUpdateMethod.Post : HttpUpdateMethod.Get;
    }

    private static int ParseInt(string raw, int fallback)
    {
        return int.TryParse(raw, out var value) ? value : fallback;
    }

    private static Dictionary<string, string> ParseHeaders(string raw)
    {
        var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var lines = raw.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        foreach (var line in lines)
        {
            var index = line.IndexOf(':');
            if (index <= 0)
            {
                continue;
            }

            var key = line[..index].Trim();
            var value = line[(index + 1)..].Trim();
            if (string.IsNullOrWhiteSpace(key))
            {
                continue;
            }

            headers[key] = value;
        }

        return headers;
    }

    private static DdnsProfile CreateDefault(int defaultInterval)
    {
        return new DdnsProfile
        {
            Id = Guid.NewGuid(),
            Name = "新配置",
            IntervalMinutes = defaultInterval > 0 ? defaultInterval : 10,
            EnableIPv4 = true,
            EnableIPv6 = false,
            ProviderType = DdnsProviderType.Cloudflare,
            Cloudflare = new CloudflareOptions
            {
                Ttl = 1,
                Proxied = false
            },
            Aliyun = new AliyunOptions
            {
                Ttl = 600
            },
            TencentCloud = new TencentCloudOptions
            {
                Ttl = 600
            },
            Oray = new OrayOptions(),
            NoIP = new NoIPOptions(),
            DuckDNS = new DuckDnsOptions(),
            GenericHttp = new GenericHttpOptions(),
            Secrets = new ProfileSecrets(),
            RuntimeStatus = new ProfileRuntimeStatus()
        };
    }

    private static DdnsProfile Clone(DdnsProfile source)
    {
        return new DdnsProfile
        {
            Id = source.Id,
            Name = source.Name,
            IsEnabled = source.IsEnabled,
            ProviderType = source.ProviderType,
            Hostname = source.Hostname,
            EnableIPv4 = source.EnableIPv4,
            EnableIPv6 = source.EnableIPv6,
            IntervalMinutes = source.IntervalMinutes,
            Cloudflare = new CloudflareOptions
            {
                ZoneName = source.Cloudflare.ZoneName,
                Ttl = source.Cloudflare.Ttl,
                Proxied = source.Cloudflare.Proxied
            },
            Aliyun = new AliyunOptions
            {
                DomainName = source.Aliyun.DomainName,
                Ttl = source.Aliyun.Ttl
            },
            TencentCloud = new TencentCloudOptions
            {
                Domain = source.TencentCloud.Domain,
                SubDomain = source.TencentCloud.SubDomain,
                Ttl = source.TencentCloud.Ttl
            },
            Oray = new OrayOptions
            {
                Domain = source.Oray.Domain,
                SubDomain = source.Oray.SubDomain
            },
            NoIP = new NoIPOptions
            {
                Hostname = source.NoIP.Hostname
            },
            DuckDNS = new DuckDnsOptions
            {
                Domain = source.DuckDNS.Domain
            },
            GenericHttp = new GenericHttpOptions
            {
                UrlTemplate = source.GenericHttp.UrlTemplate,
                Method = source.GenericHttp.Method,
                JsonBodyTemplate = source.GenericHttp.JsonBodyTemplate
            },
            Secrets = new ProfileSecrets
            {
                CloudflareApiToken = source.Secrets.CloudflareApiToken,
                AliyunAccessKeyId = source.Secrets.AliyunAccessKeyId,
                AliyunAccessKeySecret = source.Secrets.AliyunAccessKeySecret,
                TencentCloudSecretId = source.Secrets.TencentCloudSecretId,
                TencentCloudSecretKey = source.Secrets.TencentCloudSecretKey,
                OrayUsername = source.Secrets.OrayUsername,
                OrayPassword = source.Secrets.OrayPassword,
                NoIPUsername = source.Secrets.NoIPUsername,
                NoIPPassword = source.Secrets.NoIPPassword,
                DuckDnsToken = source.Secrets.DuckDnsToken,
                GenericHeaders = source.Secrets.GenericHeaders.ToDictionary(x => x.Key, x => x.Value, StringComparer.OrdinalIgnoreCase)
            },
            RuntimeStatus = new ProfileRuntimeStatus
            {
                CurrentIPv4 = source.RuntimeStatus.CurrentIPv4,
                CurrentIPv6 = source.RuntimeStatus.CurrentIPv6,
                LastResult = source.RuntimeStatus.LastResult,
                LastUpdatedAt = source.RuntimeStatus.LastUpdatedAt,
                IsRunning = source.RuntimeStatus.IsRunning
            }
        };
    }

    private static string Trim(string raw, int maxLength)
    {
        return raw.Length <= maxLength ? raw : raw[..maxLength];
    }
}


