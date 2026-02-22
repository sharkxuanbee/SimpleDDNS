namespace SimpleDDNS.Core.Models;

public sealed record IpLookupResult(
    IpProtocol Protocol,
    bool Success,
    string Address,
    string Endpoint,
    string Error)
{
    public static IpLookupResult Ok(IpProtocol protocol, string address, string endpoint)
        => new(protocol, true, address, endpoint, string.Empty);

    public static IpLookupResult Fail(IpProtocol protocol, string error)
        => new(protocol, false, string.Empty, string.Empty, error);
}
