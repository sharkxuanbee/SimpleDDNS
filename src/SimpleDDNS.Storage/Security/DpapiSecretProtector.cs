﻿using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;

namespace SimpleDDNS.Storage.Security;

public sealed class DpapiSecretProtector
{
    private const int CryptProtectUiForbidden = 0x1;
    private static readonly byte[] HardcodedEntropy = Encoding.UTF8.GetBytes("SimpleDDNS.v1.dpapi");
    private readonly string _entropyFilePath;
    private readonly Lazy<byte[]?> _fileEntropy;

    public DpapiSecretProtector(string? entropyFilePath = null)
    {
        _entropyFilePath = string.IsNullOrWhiteSpace(entropyFilePath) ? GetDefaultEntropyFilePath() : entropyFilePath;
        _fileEntropy = new Lazy<byte[]?>(LoadOrCreateFileEntropy);
    }

    private static string GetDefaultEntropyFilePath()
    {
        var folder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SimpleDDNS");
        return Path.Combine(folder, "entropy.bin");
    }

    private byte[] GetEntropy()
    {
        var fileEntropy = _fileEntropy.Value;
        if (fileEntropy is null || fileEntropy.Length == 0)
        {
            return HardcodedEntropy;
        }

        var combined = new byte[HardcodedEntropy.Length + fileEntropy.Length];
        Buffer.BlockCopy(HardcodedEntropy, 0, combined, 0, HardcodedEntropy.Length);
        Buffer.BlockCopy(fileEntropy, 0, combined, HardcodedEntropy.Length, fileEntropy.Length);
        return combined;
    }

    private byte[]? LoadOrCreateFileEntropy()
    {
        try
        {
            if (File.Exists(_entropyFilePath))
            {
                var bytes = File.ReadAllBytes(_entropyFilePath);
                if (bytes.Length == 32)
                {
                    return bytes;
                }
            }

            var newEntropy = new byte[32];
            RandomNumberGenerator.Fill(newEntropy);
            var folder = Path.GetDirectoryName(_entropyFilePath);
            if (!string.IsNullOrWhiteSpace(folder))
            {
                Directory.CreateDirectory(folder);
            }
            File.WriteAllBytes(_entropyFilePath, newEntropy);
            return newEntropy;
        }
        catch
        {
            return null;
        }
    }

    public string Protect(string plainText)
    {
        if (string.IsNullOrEmpty(plainText))
        {
            return string.Empty;
        }

        var inputBytes = Encoding.UTF8.GetBytes(plainText);
        var entropyBlob = CreateBlob(GetEntropy());
        var inputBlob = CreateBlob(inputBytes);

        try
        {
            if (!CryptProtectData(ref inputBlob, null, ref entropyBlob, IntPtr.Zero, IntPtr.Zero, CryptProtectUiForbidden, out var outputBlob))
            {
                throw CreateWin32Exception("CryptProtectData");
            }

            try
            {
                var outputBytes = BlobToByteArray(outputBlob);
                return Convert.ToBase64String(outputBytes);
            }
            finally
            {
                if (outputBlob.pbData != IntPtr.Zero)
                {
                    LocalFree(outputBlob.pbData);
                }
            }
        }
        finally
        {
            FreeBlob(inputBlob);
            FreeBlob(entropyBlob);
        }
    }

    public string Unprotect(string cipherText)
    {
        if (string.IsNullOrEmpty(cipherText))
        {
            return string.Empty;
        }

        var inputBytes = Convert.FromBase64String(cipherText);
        var inputBlob = CreateBlob(inputBytes);

        try
        {
            var entropy = GetEntropy();
            var entropyBlob = CreateBlob(entropy);
            try
            {
                if (CryptUnprotectData(ref inputBlob, IntPtr.Zero, ref entropyBlob, IntPtr.Zero, IntPtr.Zero, CryptProtectUiForbidden, out var outputBlob))
                {
                    try
                    {
                        var outputBytes = BlobToByteArray(outputBlob);
                        return Encoding.UTF8.GetString(outputBytes);
                    }
                    finally
                    {
                        if (outputBlob.pbData != IntPtr.Zero)
                        {
                            LocalFree(outputBlob.pbData);
                        }
                    }
                }
            }
            finally
            {
                FreeBlob(entropyBlob);
            }

            if (_fileEntropy.Value is not null)
            {
                var fallbackEntropyBlob = CreateBlob(HardcodedEntropy);
                try
                {
                    if (!CryptUnprotectData(ref inputBlob, IntPtr.Zero, ref fallbackEntropyBlob, IntPtr.Zero, IntPtr.Zero, CryptProtectUiForbidden, out var fallbackOutputBlob))
                    {
                        throw CreateWin32Exception("CryptUnprotectData");
                    }

                    try
                    {
                        var outputBytes = BlobToByteArray(fallbackOutputBlob);
                        return Encoding.UTF8.GetString(outputBytes);
                    }
                    finally
                    {
                        if (fallbackOutputBlob.pbData != IntPtr.Zero)
                        {
                            LocalFree(fallbackOutputBlob.pbData);
                        }
                    }
                }
                finally
                {
                    FreeBlob(fallbackEntropyBlob);
                }
            }

            throw CreateWin32Exception("CryptUnprotectData");
        }
        finally
        {
            FreeBlob(inputBlob);
        }
    }

    private static DataBlob CreateBlob(byte[] bytes)
    {
        var blob = new DataBlob
        {
            cbData = bytes.Length,
            pbData = Marshal.AllocHGlobal(bytes.Length)
        };

        Marshal.Copy(bytes, 0, blob.pbData, bytes.Length);
        return blob;
    }

    private static byte[] BlobToByteArray(DataBlob blob)
    {
        var data = new byte[blob.cbData];
        Marshal.Copy(blob.pbData, data, 0, blob.cbData);
        return data;
    }

    private static void FreeBlob(DataBlob blob)
    {
        if (blob.pbData != IntPtr.Zero)
        {
            Marshal.FreeHGlobal(blob.pbData);
        }
    }

    private static InvalidOperationException CreateWin32Exception(string operation)
    {
        var code = Marshal.GetLastWin32Error();
        return new InvalidOperationException($"{operation} failed with Win32 error: {code}");
    }

    [DllImport("Crypt32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool CryptProtectData(
        ref DataBlob pDataIn,
        string? szDataDescr,
        ref DataBlob pOptionalEntropy,
        IntPtr pvReserved,
        IntPtr pPromptStruct,
        int dwFlags,
        out DataBlob pDataOut);

    [DllImport("Crypt32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool CryptUnprotectData(
        ref DataBlob pDataIn,
        IntPtr ppszDataDescr,
        ref DataBlob pOptionalEntropy,
        IntPtr pvReserved,
        IntPtr pPromptStruct,
        int dwFlags,
        out DataBlob pDataOut);

    [DllImport("Kernel32.dll", SetLastError = true)]
    private static extern IntPtr LocalFree(IntPtr hMem);

    [StructLayout(LayoutKind.Sequential)]
    private struct DataBlob
    {
        public int cbData;
        public IntPtr pbData;
    }
}
