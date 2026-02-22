namespace SimpleDDNS.Core.Models;

public sealed class TencentCloudOptions
{
    public string SecretId { get; set; } = string.Empty;
    
    public string SecretKey { get; set; } = string.Empty;
    
    public string Domain { get; set; } = string.Empty;
    
    public string SubDomain { get; set; } = string.Empty;
    
    public int Ttl { get; set; } = 600;
}