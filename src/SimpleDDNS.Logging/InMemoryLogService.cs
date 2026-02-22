using System.Collections.Concurrent;
using System.Text;

namespace SimpleDDNS.Logging;

public sealed class InMemoryLogService : ILogService
{
    private readonly ConcurrentQueue<LogEntry> _entries = new();
    private readonly int _maxEntries;

    public InMemoryLogService(int maxEntries = 2000)
    {
        _maxEntries = maxEntries;
    }

    public event EventHandler<LogEntry>? LogReceived;

    public IReadOnlyList<LogEntry> GetEntries()
    {
        return _entries.ToArray();
    }

    public void Log(LogLevel level, string message, string? profileName = null, Exception? exception = null)
    {
        var details = exception?.ToString();
        var entry = new LogEntry(DateTimeOffset.Now, level, message, profileName, details);
        _entries.Enqueue(entry);

        while (_entries.Count > _maxEntries && _entries.TryDequeue(out _))
        {
        }

        LogReceived?.Invoke(this, entry);
    }

    public async Task ExportAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var lines = _entries
            .Select(x => $"{x.Timestamp:yyyy-MM-dd HH:mm:ss.fff}\t{x.Level}\t{x.ProfileName ?? "GLOBAL"}\t{x.Message}\t{x.Details}");

        await File.WriteAllLinesAsync(filePath, lines, Encoding.UTF8, cancellationToken).ConfigureAwait(false);
    }
}
