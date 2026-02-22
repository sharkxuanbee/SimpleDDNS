using SimpleDDNS.Core.Models;
using SimpleDDNS.Core.Services;

namespace SimpleDDNS.Tests;

public class IpAddressParserTests
{
    [Fact]
    public void TryParse_Ipv6Response_Succeeds()
    {
        var ok = IpAddressParser.TryParse("2001:db8::100\n", IpProtocol.IPv6, out var parsed, out var error);

        Assert.True(ok);
        Assert.Equal("2001:db8::100", parsed);
        Assert.Equal(string.Empty, error);
    }

    [Fact]
    public void TryParse_Ipv6ExpectedButIpv4Returned_Fails()
    {
        var ok = IpAddressParser.TryParse("198.51.100.10", IpProtocol.IPv6, out _, out var error);

        Assert.False(ok);
        Assert.Contains("期望 IPv6", error);
    }
}
