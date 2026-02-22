namespace SimpleDDNS.Core.Models;

public sealed class GenericHttpOptions
{
    public string UrlTemplate { get; set; } = string.Empty;

    public HttpUpdateMethod Method { get; set; } = HttpUpdateMethod.Get;

    public string JsonBodyTemplate { get; set; } = "{\"ip\":\"{ip}\",\"hostname\":\"{hostname}\",\"type\":\"{type}\"}";
}
