namespace SimpleDDNS.Logging;

public interface ILogService
{
    event EventHandler<LogEntry>? LogReceived;

    IReadOnlyList<LogEntry> GetEntries();

    void Log(LogLevel level, string message, string? profileName = null, Exception? exception = null);

    Task ExportAsync(string filePath, CancellationToken cancellationToken = default);
}
