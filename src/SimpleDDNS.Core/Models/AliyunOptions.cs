namespace SimpleDDNS.Core.Models;

public sealed class AliyunOptions
{
    public string AccessKeyId { get; set; } = string.Empty;
    
    public string AccessKeySecret { get; set; } = string.Empty;
    
    public string DomainName { get; set; } = string.Empty;
    
    public int Ttl { get; set; } = 600;
}