namespace SimpleDDNS.Core.Models;

public sealed class DuckDnsOptions
{
    public string Token { get; set; } = string.Empty;
    
    public string Domain { get; set; } = string.Empty;
}