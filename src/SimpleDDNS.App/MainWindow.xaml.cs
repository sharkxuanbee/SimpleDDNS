﻿﻿﻿using System.Collections.ObjectModel;
using System.Windows;
using Microsoft.Win32;
using SimpleDDNS.App.Localization;
using SimpleDDNS.App.Services;
using SimpleDDNS.App.Windows;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Core.Services;
using SimpleDDNS.Logging;
using SimpleDDNS.Providers.Providers;
using SimpleDDNS.Storage;
using MessageBox = System.Windows.MessageBox;
using SaveFileDialog = Microsoft.Win32.SaveFileDialog;

namespace SimpleDDNS.App;

public partial class MainWindow : Window
{
    private readonly InMemoryLogService _logService;
    private readonly JsonConfigurationStore _configurationStore;
    private readonly DdnsScheduler _scheduler;
    private readonly PublicIpLookupService _ipLookupService;
    private readonly Dictionary<DdnsProviderType, IDdnsProvider> _providers;
    private readonly StartupRegistrationService _startupRegistrationService;
    private readonly TrayIconService _trayIconService;

    private AppConfiguration _configuration = new();
    private ObservableCollection<DdnsProfile> _profiles = new();
    private ObservableCollection<LogEntry> _logs = new();

    private bool _exitRequested;
    private bool _isLoaded;
    private bool _isDisposed;

    public MainWindow()
    {
        InitializeComponent();

        Title = UiText.AppTitle;

        _logService = new InMemoryLogService();
        _startupRegistrationService = new StartupRegistrationService();
        _trayIconService = new TrayIconService();
        _ipLookupService = new PublicIpLookupService(_logService);

        var providerInstances = new IDdnsProvider[]
        {
            new CloudflareProvider(_logService),
            new GenericHttpProvider()
        };

        _providers = providerInstances.ToDictionary(x => x.ProviderType, x => x);
        _scheduler = new DdnsScheduler(_ipLookupService, providerInstances, _logService);
        _configurationStore = new JsonConfigurationStore(_logService);

        ProfilesDataGrid.ItemsSource = _profiles;
        LogsDataGrid.ItemsSource = _logs;

        ConfigPathTextBlock.Text = $"配置文件: {_configurationStore.ConfigurationPath}";

        Loaded += MainWindow_OnLoaded;
        Closing += MainWindow_OnClosing;

        _logService.LogReceived += LogService_OnLogReceived;
        _scheduler.ProfileStatusChanged += Scheduler_OnProfileStatusChanged;

        _trayIconService.OpenRequested += TrayIconService_OnOpenRequested;
        _trayIconService.ToggleSchedulerRequested += TrayIconService_OnToggleSchedulerRequested;
        _trayIconService.RunNowRequested += TrayIconService_OnRunNowRequested;
        _trayIconService.ExitRequested += TrayIconService_OnExitRequested;
    }

    protected override void OnClosed(EventArgs e)
    {
        base.OnClosed(e);
        CleanupAsync().GetAwaiter().GetResult();
    }

    private async void MainWindow_OnLoaded(object sender, RoutedEventArgs e)
    {
        if (_isLoaded)
        {
            return;
        }

        _isLoaded = true;
        _trayIconService.Show();

        try
        {
            _configuration = await _configurationStore.LoadAsync().ConfigureAwait(true);
            _profiles = new ObservableCollection<DdnsProfile>(_configuration.Profiles);
            _logs = new ObservableCollection<LogEntry>(_logService.GetEntries());

            ProfilesDataGrid.ItemsSource = _profiles;
            LogsDataGrid.ItemsSource = _logs;

            await _scheduler.SetConfigurationAsync(_configuration).ConfigureAwait(true);
            _startupRegistrationService.SetEnabled(_configuration.Settings.StartWithWindows);

            UpdateSchedulerUi();
            _logService.Log(LogLevel.Information, "应用已启动");
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "初始化失败", exception: ex);
            MessageBox.Show(this, ex.Message, "初始化失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void MainWindow_OnClosing(object? sender, System.ComponentModel.CancelEventArgs e)
    {
        if (!_exitRequested && _configuration.Settings.MinimizeToTrayOnClose)
        {
            e.Cancel = true;
            Hide();
            return;
        }

        await _scheduler.StopAsync().ConfigureAwait(true);
    }

    private void AddProfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var wizard = new ProfileWizardWindow(
            existing: null,
            _configuration.Settings.DefaultIntervalMinutes,
            TestProviderAsync)
        {
            Owner = this
        };

        if (wizard.ShowDialog() != true)
        {
            return;
        }

        _profiles.Add(wizard.ResultProfile);
        _ = SaveAndApplyConfigurationAsync();
    }

    private void EditProfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var selected = GetSelectedProfile();
        if (selected is null)
        {
            return;
        }

        var wizard = new ProfileWizardWindow(
            existing: selected,
            _configuration.Settings.DefaultIntervalMinutes,
            TestProviderAsync)
        {
            Owner = this
        };

        if (wizard.ShowDialog() != true)
        {
            return;
        }

        ApplyEditedProfile(selected, wizard.ResultProfile);
        _ = SaveAndApplyConfigurationAsync();
    }

    private void DeleteProfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var selected = GetSelectedProfile();
        if (selected is null)
        {
            return;
        }

        var confirm = MessageBox.Show(this, $"确认删除配置 {selected.Name} ?", "确认", MessageBoxButton.YesNo, MessageBoxImage.Question);
        if (confirm != MessageBoxResult.Yes)
        {
            return;
        }

        _profiles.Remove(selected);
        _ = SaveAndApplyConfigurationAsync();
    }

    private void EnableProfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var selected = GetSelectedProfile();
        if (selected is null)
        {
            return;
        }

        selected.IsEnabled = true;
        _ = SaveAndApplyConfigurationAsync();
    }

    private void DisableProfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var selected = GetSelectedProfile();
        if (selected is null)
        {
            return;
        }

        selected.IsEnabled = false;
        _ = SaveAndApplyConfigurationAsync();
    }

    private async void StartSchedulerButton_OnClick(object sender, RoutedEventArgs e)
    {
        try
        {
            await _scheduler.StartAsync().ConfigureAwait(true);
            UpdateSchedulerUi();
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "启动调度器失败", exception: ex);
            MessageBox.Show(this, ex.Message, "启动失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void StopSchedulerButton_OnClick(object sender, RoutedEventArgs e)
    {
        try
        {
            await _scheduler.StopAsync().ConfigureAwait(true);
            UpdateSchedulerUi();
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "停止调度器失败", exception: ex);
            MessageBox.Show(this, ex.Message, "停止失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void RunNowButton_OnClick(object sender, RoutedEventArgs e)
    {
        try
        {
            await _scheduler.TriggerAllNowAsync().ConfigureAwait(true);
            ProfilesDataGrid.Items.Refresh();
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "立即更新失败", exception: ex);
            MessageBox.Show(this, ex.Message, "更新失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void RunSelectedNowButton_OnClick(object sender, RoutedEventArgs e)
    {
        var selected = GetSelectedProfile();
        if (selected is null)
        {
            return;
        }

        try
        {
            await _scheduler.TriggerNowAsync(selected.Id).ConfigureAwait(true);
            ProfilesDataGrid.Items.Refresh();
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "更新选中配置失败", selected.Name, ex);
            MessageBox.Show(this, ex.Message, "更新失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void SettingsButton_OnClick(object sender, RoutedEventArgs e)
    {
        var window = new SettingsWindow(_configuration.Settings)
        {
            Owner = this
        };

        if (window.ShowDialog() != true)
        {
            return;
        }

        _configuration.Settings = window.ResultSettings;
        _startupRegistrationService.SetEnabled(_configuration.Settings.StartWithWindows);

        _ = SaveAndApplyConfigurationAsync();
    }

    private async void ImportConfigButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new Microsoft.Win32.OpenFileDialog
        {
            Filter = "JSON 文件 (*.json)|*.json",
            Title = "导入配置"
        };

        if (dialog.ShowDialog(this) != true)
        {
            return;
        }

        var confirm = MessageBox.Show(
            this,
            "导入将覆盖当前所有配置（包括 Profile 和设置）。\n\n注意：如果导入文件来自其他机器，且包含加密的敏感信息（如密码/Key），导入后这些信息将无法解密（需重新输入）。\n\n是否继续？",
            "确认导入",
            MessageBoxButton.YesNo,
            MessageBoxImage.Warning);

        if (confirm != MessageBoxResult.Yes)
        {
            return;
        }

        try
        {
            var newConfig = await _configurationStore.LoadFromPathAsync(dialog.FileName).ConfigureAwait(true);
            
            _configuration = newConfig;
            _profiles = new ObservableCollection<DdnsProfile>(_configuration.Profiles);
            ProfilesDataGrid.ItemsSource = _profiles;
            
            _startupRegistrationService.SetEnabled(_configuration.Settings.StartWithWindows);
            
            await SaveAndApplyConfigurationAsync().ConfigureAwait(true);
            
            MessageBox.Show(this, "导入成功", "完成", MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "导入配置失败", exception: ex);
            MessageBox.Show(this, ex.Message, "导入失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void ExportConfigButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new SaveFileDialog
        {
            Filter = "JSON 文件 (*.json)|*.json",
            FileName = "simpleddns-export.json"
        };

        if (dialog.ShowDialog(this) != true)
        {
            return;
        }

        var includeSensitive = false;
        var firstConfirm = MessageBox.Show(
            this,
            "默认不导出敏感字段。是否包含敏感字段（加密形式）？",
            "导出配置",
            MessageBoxButton.YesNoCancel,
            MessageBoxImage.Question);

        if (firstConfirm == MessageBoxResult.Cancel)
        {
            return;
        }

        if (firstConfirm == MessageBoxResult.Yes)
        {
            var secondConfirm = MessageBox.Show(
                this,
                "确认导出敏感字段？请妥善保管导出文件。",
                "二次确认",
                MessageBoxButton.YesNo,
                MessageBoxImage.Warning);

            if (secondConfirm != MessageBoxResult.Yes)
            {
                return;
            }

            includeSensitive = true;
        }

        try
        {
            var config = BuildConfigurationSnapshot();
            await _configurationStore.ExportAsync(config, dialog.FileName, includeSensitive).ConfigureAwait(true);
            MessageBox.Show(this, "导出成功", "完成", MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "导出配置失败", exception: ex);
            MessageBox.Show(this, ex.Message, "导出失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private async void ExportLogsButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new SaveFileDialog
        {
            Filter = "日志文件 (*.log)|*.log|文本文件 (*.txt)|*.txt",
            FileName = $"simpleddns-{DateTime.Now:yyyyMMdd-HHmmss}.log"
        };

        if (dialog.ShowDialog(this) != true)
        {
            return;
        }

        try
        {
            await _logService.ExportAsync(dialog.FileName).ConfigureAwait(true);
            MessageBox.Show(this, "日志导出成功", "完成", MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "导出日志失败", exception: ex);
            MessageBox.Show(this, ex.Message, "导出失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void TrayIconService_OnOpenRequested(object? sender, EventArgs e)
    {
        Dispatcher.Invoke(() =>
        {
            Show();
            WindowState = WindowState.Normal;
            Activate();
        });
    }

    private void TrayIconService_OnToggleSchedulerRequested(object? sender, EventArgs e)
    {
        _ = Dispatcher.InvokeAsync(async () =>
        {
            if (_scheduler.IsRunning)
            {
                await _scheduler.StopAsync().ConfigureAwait(true);
            }
            else
            {
                await _scheduler.StartAsync().ConfigureAwait(true);
            }

            UpdateSchedulerUi();
        });
    }

    private void TrayIconService_OnRunNowRequested(object? sender, EventArgs e)
    {
        _ = Dispatcher.InvokeAsync(async () =>
        {
            await _scheduler.TriggerAllNowAsync().ConfigureAwait(true);
            ProfilesDataGrid.Items.Refresh();
        });
    }

    private void TrayIconService_OnExitRequested(object? sender, EventArgs e)
    {
        Dispatcher.Invoke(() =>
        {
            _exitRequested = true;
            Close();
        });
    }

    private async Task SaveAndApplyConfigurationAsync()
    {
        try
        {
            _configuration = BuildConfigurationSnapshot();
            await _configurationStore.SaveAsync(_configuration).ConfigureAwait(true);
            await _scheduler.SetConfigurationAsync(_configuration).ConfigureAwait(true);
            ProfilesDataGrid.Items.Refresh();
        }
        catch (Exception ex)
        {
            _logService.Log(LogLevel.Error, "保存配置失败", exception: ex);
            MessageBox.Show(this, ex.Message, "保存失败", MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private AppConfiguration BuildConfigurationSnapshot()
    {
        return new AppConfiguration
        {
            Settings = _configuration.Settings,
            Profiles = _profiles.ToList()
        };
    }

    private DdnsProfile? GetSelectedProfile()
    {
        return ProfilesDataGrid.SelectedItem as DdnsProfile;
    }

    private async Task<ProviderResult> TestProviderAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (!_providers.TryGetValue(profile.ProviderType, out var provider))
        {
            return new ProviderResult(false, "未找到 Provider");
        }

        return await provider.TestAsync(profile, cancellationToken).ConfigureAwait(false);
    }

    private void ApplyEditedProfile(DdnsProfile target, DdnsProfile source)
    {
        var runtime = target.RuntimeStatus;

        target.Name = source.Name;
        target.ProviderType = source.ProviderType;
        target.Hostname = source.Hostname;
        target.EnableIPv4 = source.EnableIPv4;
        target.EnableIPv6 = source.EnableIPv6;
        target.IntervalMinutes = source.IntervalMinutes;
        target.Cloudflare = source.Cloudflare;
        target.GenericHttp = source.GenericHttp;
        target.Secrets = source.Secrets;

        target.RuntimeStatus = runtime;
    }

    private void UpdateSchedulerUi()
    {
        SchedulerStatusTextBlock.Text = _scheduler.IsRunning ? "调度状态: 运行中" : "调度状态: 未启动";
        _trayIconService.UpdateSchedulerState(_scheduler.IsRunning);
    }

    private void Scheduler_OnProfileStatusChanged(object? sender, ProfileStatusChangedEventArgs e)
    {
        Dispatcher.Invoke(() =>
        {
            ProfilesDataGrid.Items.Refresh();
        });
    }

    private void LogService_OnLogReceived(object? sender, LogEntry entry)
    {
        Dispatcher.Invoke(() =>
        {
            _logs.Add(entry);
            while (_logs.Count > 2000)
            {
                _logs.RemoveAt(0);
            }

            if (_logs.Count > 0)
            {
                LogsDataGrid.ScrollIntoView(_logs[^1]);
            }
        });
    }

    private async Task CleanupAsync()
    {
        if (_isDisposed)
        {
            return;
        }

        _isDisposed = true;

        _logService.LogReceived -= LogService_OnLogReceived;
        _scheduler.ProfileStatusChanged -= Scheduler_OnProfileStatusChanged;

        _trayIconService.OpenRequested -= TrayIconService_OnOpenRequested;
        _trayIconService.ToggleSchedulerRequested -= TrayIconService_OnToggleSchedulerRequested;
        _trayIconService.RunNowRequested -= TrayIconService_OnRunNowRequested;
        _trayIconService.ExitRequested -= TrayIconService_OnExitRequested;

        await _scheduler.StopAsync().ConfigureAwait(false);
        await _scheduler.DisposeAsync().ConfigureAwait(false);
        _ipLookupService.Dispose();

        foreach (var disposable in _providers.Values.OfType<IDisposable>())
        {
            disposable.Dispose();
        }

        _trayIconService.Dispose();
    }
}
