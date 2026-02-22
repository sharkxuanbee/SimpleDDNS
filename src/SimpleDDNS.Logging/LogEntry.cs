namespace SimpleDDNS.Logging;

public sealed record LogEntry(
    DateTimeOffset Timestamp,
    LogLevel Level,
    string Message,
    string? ProfileName = null,
    string? Details = null);
