namespace SimpleDDNS.Core.Models;

public sealed record ProviderResult(bool Success, string Message, string RawResponse = "");
