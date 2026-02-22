﻿using System.Collections.Concurrent;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Core.Services;

public sealed class DdnsScheduler : IAsyncDisposable
{
    private readonly IIpLookupService _ipLookupService;
    private readonly Dictionary<DdnsProviderType, IDdnsProvider> _providers;
    private readonly ILogService _logService;
    private readonly SemaphoreSlim _sync = new(1, 1);
    private readonly ConcurrentDictionary<Guid, WorkerState> _workers = new();

    private List<DdnsProfile> _profiles = new();
    private AppSettings _settings = new();
    private CancellationTokenSource? _globalCts;

    public DdnsScheduler(IIpLookupService ipLookupService, IEnumerable<IDdnsProvider> providers, ILogService logService)
    {
        _ipLookupService = ipLookupService;
        _providers = providers.ToDictionary(x => x.ProviderType, x => x);
        _logService = logService;
    }

    public bool IsRunning { get; private set; }

    public event EventHandler<ProfileStatusChangedEventArgs>? ProfileStatusChanged;

    public async Task SetConfigurationAsync(AppConfiguration configuration, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(configuration);

        await _sync.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            _profiles = configuration.Profiles;
            _settings = configuration.Settings;

            if (IsRunning)
            {
                ReconcileWorkers();
            }
        }
        finally
        {
            _sync.Release();
        }
    }

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        await _sync.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (IsRunning)
            {
                return;
            }

            _globalCts = new CancellationTokenSource();
            IsRunning = true;
            ReconcileWorkers();
            _logService.Log(LogLevel.Information, "后台调度已启动");
        }
        finally
        {
            _sync.Release();
        }
    }

    public async Task StopAsync(CancellationToken cancellationToken = default)
    {
        List<Task> runningTasks;
        await _sync.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (!IsRunning)
            {
                return;
            }

            IsRunning = false;
            _globalCts?.Cancel();

            runningTasks = _workers.Values.Select(x => x.LoopTask).ToList();
            _workers.Clear();
            _logService.Log(LogLevel.Information, "正在停止后台调度...");
        }
        finally
        {
            _sync.Release();
        }

        await Task.WhenAll(runningTasks).ConfigureAwait(false);
        _logService.Log(LogLevel.Information, "后台调度已停止");
    }

    public async Task TriggerNowAsync(Guid profileId, CancellationToken cancellationToken = default)
    {
        var profile = _profiles.FirstOrDefault(x => x.Id == profileId);
        if (profile is null)
        {
            return;
        }

        await ExecuteProfileAsync(profile, cancellationToken).ConfigureAwait(false);
    }

    public async Task TriggerAllNowAsync(CancellationToken cancellationToken = default)
    {
        var enabled = _profiles.Where(x => x.IsEnabled).ToList();
        await Task.WhenAll(enabled.Select(x => ExecuteProfileAsync(x, cancellationToken))).ConfigureAwait(false);
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            await StopAsync().ConfigureAwait(false);
        }
        catch
        {
        }

        _sync.Dispose();
    }

    private void ReconcileWorkers()
    {
        var enabledProfiles = _profiles.Where(x => x.IsEnabled).ToDictionary(x => x.Id, x => x);

        foreach (var existing in _workers.Keys)
        {
            if (enabledProfiles.ContainsKey(existing))
            {
                continue;
            }

            if (_workers.TryRemove(existing, out var worker))
            {
                worker.Cancellation.Cancel();
            }
        }

        if (!IsRunning || _globalCts is null)
        {
            return;
        }

        foreach (var profile in enabledProfiles.Values)
        {
            if (_workers.TryGetValue(profile.Id, out var workerState))
            {
                workerState.Profile = profile;
                continue;
            }

            var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(_globalCts.Token);
            var worker = new WorkerState(profile, linkedCts);
            worker.LoopTask = RunWorkerLoopAsync(worker);
            _workers[profile.Id] = worker;
        }
    }

    private async Task RunWorkerLoopAsync(WorkerState worker)
    {
        while (!worker.Cancellation.IsCancellationRequested)
        {
            try
            {
                await ExecuteProfileAsync(worker.Profile, worker.Cancellation.Token).ConfigureAwait(false);
                var delay = TimeSpan.FromMinutes(Math.Max(1, worker.Profile.IntervalMinutes));
                await Task.Delay(delay, worker.Cancellation.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (worker.Cancellation.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex)
            {
                _logService.Log(LogLevel.Error, "Profile 循环任务发生未处理异常", worker.Profile.Name, ex);
                try
                {
                    await Task.Delay(TimeSpan.FromSeconds(10), worker.Cancellation.Token).ConfigureAwait(false);
                }
                catch (OperationCanceledException)
                {
                    break;
                }
            }
        }
    }

    private async Task ExecuteProfileAsync(DdnsProfile profile, CancellationToken cancellationToken)
    {
        if (!_providers.TryGetValue(profile.ProviderType, out var provider))
        {
            profile.RuntimeStatus.LastResult = $"未找到 Provider: {profile.ProviderType}";
            RaiseStatus(profile);
            return;
        }

        if (!await profile.RuntimeStatus.RunGate.WaitAsync(0, cancellationToken).ConfigureAwait(false))
        {
            _logService.Log(LogLevel.Debug, "跳过并发执行请求", profile.Name);
            return;
        }

        try
        {
            profile.RuntimeStatus.IsRunning = true;
            RaiseStatus(profile);

            var messages = new List<string>();
            var successIPv4 = true;
            var successIPv6 = true;

            if (!profile.EnableIPv4 && !profile.EnableIPv6)
            {
                profile.RuntimeStatus.LastResult = "IPv4/IPv6 都未启用";
                RaiseStatus(profile);
                return;
            }

            if (profile.EnableIPv4)
            {
                successIPv4 = await ProcessSingleProtocolAsync(profile, provider, IpProtocol.IPv4, cancellationToken, messages).ConfigureAwait(false);
            }

            if (profile.EnableIPv6)
            {
                successIPv6 = await ProcessSingleProtocolAsync(profile, provider, IpProtocol.IPv6, cancellationToken, messages).ConfigureAwait(false);
            }

            if (successIPv4 && successIPv6)
            {
                profile.RuntimeStatus.LastUpdatedAt = DateTimeOffset.Now;
            }

            profile.RuntimeStatus.LastResult = messages.Count == 0 ? "无操作" : string.Join("; ", messages);
            RaiseStatus(profile);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            profile.RuntimeStatus.LastResult = "任务已取消";
            RaiseStatus(profile);
        }
        catch (Exception ex)
        {
            profile.RuntimeStatus.LastResult = $"异常: {ex.Message}";
            _logService.Log(LogLevel.Error, "执行 Profile 失败", profile.Name, ex);
            RaiseStatus(profile);
        }
        finally
        {
            profile.RuntimeStatus.IsRunning = false;
            profile.RuntimeStatus.RunGate.Release();
            RaiseStatus(profile);
        }
    }

    private async Task<bool> ProcessSingleProtocolAsync(
        DdnsProfile profile,
        IDdnsProvider provider,
        IpProtocol protocol,
        CancellationToken cancellationToken,
        List<string> messages)
    {
        var endpoints = protocol == IpProtocol.IPv4
            ? _settings.IPv4ProbeEndpoints
            : _settings.IPv6ProbeEndpoints;

        var lookup = await LookupWithRetryAsync(profile, protocol, endpoints, cancellationToken).ConfigureAwait(false);
        if (!lookup.Success)
        {
            messages.Add($"{protocol} 不可用: {lookup.Error}");
            return false;
        }

        var currentIp = protocol == IpProtocol.IPv4
            ? profile.RuntimeStatus.CurrentIPv4
            : profile.RuntimeStatus.CurrentIPv6;

        if (string.Equals(currentIp, lookup.Address, StringComparison.OrdinalIgnoreCase))
        {
            messages.Add($"{protocol} 未变化 ({lookup.Address})");
            return true;
        }

        var recordType = protocol == IpProtocol.IPv4 ? DnsRecordType.A : DnsRecordType.AAAA;
        var providerResult = await UpdateWithRetryAsync(profile, provider, recordType, lookup.Address, cancellationToken).ConfigureAwait(false);
        if (!providerResult.Success)
        {
            messages.Add($"{recordType} 更新失败: {providerResult.Message}");
            return false;
        }

        if (protocol == IpProtocol.IPv4)
        {
            profile.RuntimeStatus.CurrentIPv4 = lookup.Address;
        }
        else
        {
            profile.RuntimeStatus.CurrentIPv6 = lookup.Address;
        }

        messages.Add($"{recordType} 已更新为 {lookup.Address}");
        _logService.Log(LogLevel.Information, $"{recordType} 更新成功: {lookup.Address}", profile.Name);
        return true;
    }

    private async Task<IpLookupResult> LookupWithRetryAsync(
        DdnsProfile profile,
        IpProtocol protocol,
        IReadOnlyList<string> endpoints,
        CancellationToken cancellationToken)
    {
        var delay = TimeSpan.FromSeconds(2);
        var result = IpLookupResult.Fail(protocol, "未执行");

        for (var attempt = 1; attempt <= 3; attempt++)
        {
            result = await _ipLookupService.LookupAsync(protocol, endpoints, cancellationToken).ConfigureAwait(false);
            if (result.Success)
            {
                _logService.Log(LogLevel.Debug, $"{protocol} 探测成功: {result.Address} ({result.Endpoint})", profile.Name);
                return result;
            }

            _logService.Log(LogLevel.Warning, $"{protocol} 探测失败 (第 {attempt}/3 次): {result.Error}", profile.Name);
            if (attempt == 3)
            {
                break;
            }

            await Task.Delay(delay, cancellationToken).ConfigureAwait(false);
            delay = TimeSpan.FromSeconds(delay.TotalSeconds * 2);
        }

        return result;
    }

    private async Task<ProviderResult> UpdateWithRetryAsync(
        DdnsProfile profile,
        IDdnsProvider provider,
        DnsRecordType recordType,
        string ipAddress,
        CancellationToken cancellationToken)
    {
        var delay = TimeSpan.FromSeconds(2);
        var result = new ProviderResult(false, "未执行");

        for (var attempt = 1; attempt <= 3; attempt++)
        {
            result = await provider.UpdateRecordAsync(profile, recordType, ipAddress, cancellationToken).ConfigureAwait(false);
            if (result.Success)
            {
                return result;
            }

            _logService.Log(LogLevel.Warning, $"{recordType} 更新失败 (第 {attempt}/3 次): {result.Message}", profile.Name);
            if (attempt == 3)
            {
                break;
            }

            await Task.Delay(delay, cancellationToken).ConfigureAwait(false);
            delay = TimeSpan.FromSeconds(delay.TotalSeconds * 2);
        }

        return result;
    }

    private void RaiseStatus(DdnsProfile profile)
    {
        ProfileStatusChanged?.Invoke(this, new ProfileStatusChangedEventArgs(profile.Id, profile.RuntimeStatus));
    }

    private sealed class WorkerState
    {
        public WorkerState(DdnsProfile profile, CancellationTokenSource cancellation)
        {
            Profile = profile;
            Cancellation = cancellation;
        }

        public DdnsProfile Profile { get; set; }

        public CancellationTokenSource Cancellation { get; }

        public Task LoopTask { get; set; } = Task.CompletedTask;
    }
}
