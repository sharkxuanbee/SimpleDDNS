using System.Net;
using SimpleDDNS.Core.Models;

namespace SimpleDDNS.Core.Services;

public static class IpAddressParser
{
    public static bool TryParse(string raw, IpProtocol protocol, out string address, out string error)
    {
        address = string.Empty;
        error = string.Empty;

        if (string.IsNullOrWhiteSpace(raw))
        {
            error = "响应为空";
            return false;
        }

        var token = raw.Trim().Split(new[] { '\r', '\n', ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault();
        if (token is null)
        {
            error = "响应中未找到 IP";
            return false;
        }

        if (!IPAddress.TryParse(token, out var ipAddress))
        {
            error = $"无法解析为 IP: {token}";
            return false;
        }

        if (protocol == IpProtocol.IPv4 && ipAddress.AddressFamily != System.Net.Sockets.AddressFamily.InterNetwork)
        {
            error = $"期望 IPv4，但得到 {ipAddress}";
            return false;
        }

        if (protocol == IpProtocol.IPv6 && ipAddress.AddressFamily != System.Net.Sockets.AddressFamily.InterNetworkV6)
        {
            error = $"期望 IPv6，但得到 {ipAddress}";
            return false;
        }

        address = ipAddress.ToString();
        return true;
    }
}
