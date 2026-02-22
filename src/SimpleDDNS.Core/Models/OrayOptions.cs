namespace SimpleDDNS.Core.Models;

public sealed class OrayOptions
{
    public string Username { get; set; } = string.Empty;
    
    public string Password { get; set; } = string.Empty;
    
    public string Domain { get; set; } = string.Empty;
    
    public string SubDomain { get; set; } = string.Empty;
}