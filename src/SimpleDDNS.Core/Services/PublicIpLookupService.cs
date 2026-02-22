﻿using System.Net;
using System.Net.Http;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using SimpleDDNS.Core.Abstractions;
using SimpleDDNS.Core.Models;
using SimpleDDNS.Logging;

namespace SimpleDDNS.Core.Services;

public sealed class PublicIpLookupService : IIpLookupService, IDisposable
{
    private readonly HttpClient _ipv4Client;
    private readonly HttpClient _ipv6Client;
    private readonly ILogService _logService;

    public PublicIpLookupService(ILogService logService, TimeSpan? timeout = null)
    {
        _logService = logService;
        var requestTimeout = timeout ?? TimeSpan.FromSeconds(15);
        _ipv4Client = CreateClient(AddressFamily.InterNetwork, requestTimeout);
        _ipv6Client = CreateClient(AddressFamily.InterNetworkV6, requestTimeout);
    }

    public async Task<IpLookupResult> LookupAsync(IpProtocol protocol, IReadOnlyList<string> endpoints, CancellationToken cancellationToken)
    {
        if (endpoints.Count == 0)
        {
            return IpLookupResult.Fail(protocol, "没有可用探测源");
        }

        var errors = new List<string>();
        var client = protocol == IpProtocol.IPv4 ? _ipv4Client : _ipv6Client;

        foreach (var endpoint in endpoints)
        {
            if (string.IsNullOrWhiteSpace(endpoint))
            {
                continue;
            }

            var normalizedEndpoint = endpoint.Trim();
            if (TryLookupFromLocalInterface(protocol, normalizedEndpoint, out var localLookup))
            {
                if (localLookup.Success)
                {
                    return localLookup;
                }

                errors.Add($"{normalizedEndpoint} -> {localLookup.Error}");
                continue;
            }

            try
            {
                using var request = new HttpRequestMessage(HttpMethod.Get, normalizedEndpoint);
                using var response = await client.SendAsync(request, cancellationToken).ConfigureAwait(false);
                var body = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
                if (!response.IsSuccessStatusCode)
                {
                    errors.Add($"{normalizedEndpoint} -> HTTP {(int)response.StatusCode}");
                    continue;
                }

                if (!IpAddressParser.TryParse(body, protocol, out var parsedIp, out var parseError))
                {
                    errors.Add($"{normalizedEndpoint} -> {parseError}");
                    continue;
                }

                return IpLookupResult.Ok(protocol, parsedIp, normalizedEndpoint);
            }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
                errors.Add($"{normalizedEndpoint} -> 超时");
            }
            catch (HttpRequestException ex)
            {
                errors.Add($"{normalizedEndpoint} -> 网络错误: {ex.Message}");
            }
            catch (SocketException ex)
            {
                errors.Add($"{normalizedEndpoint} -> Socket 错误: {ex.SocketErrorCode}");
            }
            catch (Exception ex)
            {
                errors.Add($"{normalizedEndpoint} -> 未知错误: {ex.Message}");
            }
        }

        var reason = string.Join(" | ", errors);
        var finalError = string.IsNullOrWhiteSpace(reason) ? "探测失败" : reason;
        _logService.Log(LogLevel.Warning, $"{protocol} 探测失败: {finalError}");
        return IpLookupResult.Fail(protocol, finalError);
    }

    public void Dispose()
    {
        _ipv4Client.Dispose();
        _ipv6Client.Dispose();
    }

    private static HttpClient CreateClient(AddressFamily family, TimeSpan timeout)
    {
        var handler = new SocketsHttpHandler
        {
            ConnectCallback = async (context, cancellationToken) =>
            {
                var addresses = await Dns.GetHostAddressesAsync(context.DnsEndPoint.Host, cancellationToken).ConfigureAwait(false);
                var candidate = addresses.FirstOrDefault(x => x.AddressFamily == family);
                if (candidate is null)
                {
                    throw new SocketException((int)SocketError.AddressFamilyNotSupported);
                }

                var socket = new Socket(family, SocketType.Stream, ProtocolType.Tcp)
                {
                    NoDelay = true
                };

                try
                {
                    await socket.ConnectAsync(candidate, context.DnsEndPoint.Port, cancellationToken).ConfigureAwait(false);
                    return new NetworkStream(socket, ownsSocket: true);
                }
                catch
                {
                    socket.Dispose();
                    throw;
                }
            }
        };

        return new HttpClient(handler)
        {
            Timeout = timeout
        };
    }

    private static bool TryLookupFromLocalInterface(IpProtocol protocol, string endpoint, out IpLookupResult result)
    {
        result = IpLookupResult.Fail(protocol, "未执行");
        if (!IsLocalEndpointToken(endpoint))
        {
            return false;
        }

        if (endpoint.Equals("local://ipv4", StringComparison.OrdinalIgnoreCase) && protocol != IpProtocol.IPv4)
        {
            result = IpLookupResult.Fail(protocol, "该探测源仅用于 IPv4");
            return true;
        }

        if (endpoint.Equals("local://ipv6", StringComparison.OrdinalIgnoreCase) && protocol != IpProtocol.IPv6)
        {
            result = IpLookupResult.Fail(protocol, "该探测源仅用于 IPv6");
            return true;
        }

        if (TryGetLocalAddress(protocol, out var address, out var error))
        {
            result = IpLookupResult.Ok(protocol, address, endpoint);
            return true;
        }

        result = IpLookupResult.Fail(protocol, error);
        return true;
    }

    private static bool IsLocalEndpointToken(string endpoint)
    {
        return endpoint.Equals("local://ipv4", StringComparison.OrdinalIgnoreCase)
            || endpoint.Equals("local://ipv6", StringComparison.OrdinalIgnoreCase)
            || endpoint.Equals("local://ip", StringComparison.OrdinalIgnoreCase)
            || endpoint.Equals("local://auto", StringComparison.OrdinalIgnoreCase);
    }

    private static bool TryGetLocalAddress(IpProtocol protocol, out string address, out string error)
    {
        address = string.Empty;
        error = string.Empty;

        var family = protocol == IpProtocol.IPv4
            ? AddressFamily.InterNetwork
            : AddressFamily.InterNetworkV6;

        var preferred = new List<string>();
        var fallback = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        foreach (var nic in NetworkInterface.GetAllNetworkInterfaces())
        {
            if (nic.OperationalStatus != OperationalStatus.Up)
            {
                continue;
            }

            if (nic.NetworkInterfaceType is NetworkInterfaceType.Loopback or NetworkInterfaceType.Tunnel)
            {
                continue;
            }

            IPInterfaceProperties properties;
            try
            {
                properties = nic.GetIPProperties();
            }
            catch
            {
                continue;
            }

            var hasGateway = properties.GatewayAddresses
                .Any(g => g.Address.AddressFamily == family && !IsAnyAddress(g.Address));

            foreach (var unicastAddress in properties.UnicastAddresses)
            {
                var ipAddress = unicastAddress.Address;
                if (ipAddress.AddressFamily != family)
                {
                    continue;
                }

                if (!IsUsableLocalAddress(ipAddress, protocol))
                {
                    continue;
                }

                var candidate = ipAddress.ToString();
                if (!seen.Add(candidate))
                {
                    continue;
                }

                if (hasGateway)
                {
                    preferred.Add(candidate);
                }
                else
                {
                    fallback.Add(candidate);
                }
            }
        }

        address = preferred.FirstOrDefault() ?? fallback.FirstOrDefault() ?? string.Empty;
        if (!string.IsNullOrWhiteSpace(address))
        {
            return true;
        }

        error = protocol == IpProtocol.IPv4
            ? "未找到可用本地 IPv4 地址"
            : "未找到可用本地 IPv6 地址";
        return false;
    }

    private static bool IsAnyAddress(IPAddress address)
    {
        return address.Equals(IPAddress.Any)
            || address.Equals(IPAddress.None)
            || address.Equals(IPAddress.IPv6Any)
            || address.Equals(IPAddress.IPv6None);
    }

    private static bool IsUsableLocalAddress(IPAddress address, IpProtocol protocol)
    {
        if (IPAddress.IsLoopback(address) || IsAnyAddress(address))
        {
            return false;
        }

        if (protocol == IpProtocol.IPv4)
        {
            return !IsApipaAddress(address);
        }

        return !address.IsIPv6LinkLocal && !address.IsIPv6Multicast;
    }

    private static bool IsApipaAddress(IPAddress address)
    {
        var bytes = address.GetAddressBytes();
        return bytes.Length == 4 && bytes[0] == 169 && bytes[1] == 254;
    }
}
