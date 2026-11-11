using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;

internal static class CertificateDiscovery
{
    private const string ExpectedClientSha256 =
        "E87AAAE6B4204CCED22C05975D5378A7EA49E5560C171905C450FBC848E933C1";
    private const int TextRva = 0x1000;
    private const int RdataRva = 0x489F000;
    private const int LoaderRva70170 = 0x4721670;
    private const int CallerRva70170 = 0x1F25778;
    private const int AllocatorRva70170 = 0x46027E0;
    private const int CopyRva70170 = 0x483D490;
    private const int AllocatorTagRva70170 = 0x4CFD090;
    private const int MaxImageBytes = 256 * 1024 * 1024;

    public static void Discover(string executable, string referenceTextPath, string referenceRdataPath)
    {
        string full = Path.GetFullPath(executable).Replace('/', '\\');
        if (!full.Contains("\\target\\forever-login-70450\\client\\", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetFileName(full), "WowB.exe", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Only the isolated 70450 WowB.exe is allowed");
        string hash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(full)));
        if (!hash.Equals(ExpectedClientSha256, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Unverified executable");

        byte[] text = File.ReadAllBytes(referenceTextPath);
        byte[] rdata = File.ReadAllBytes(referenceRdataPath);
        byte[] loader = Convert.FromHexString("4C8BDC53564881EC88000000");
        byte[] fileId = Convert.FromHexString("DAE17500");
        byte[] allocator = Pattern(text, AllocatorRva70170 - TextRva, 12);
        byte[] copy = Pattern(text, CopyRva70170 - TextRva, 12);
        byte[] caller = Pattern(text, CallerRva70170 - TextRva - 16, 37);
        byte[] callerMask = Enumerable.Repeat((byte)1, caller.Length).ToArray();
        for (int index = 16; index < 21; index++)
            callerMask[index] = 0;
        byte[] tag = Pattern(rdata, AllocatorTagRva70170 - RdataRva, 32);

        using Process process = LocalCertificateProvider.FindClient(full);
        nint handle = DiscoveryNative.OpenProcess(0x0410, false, process.Id);
        if (handle == 0)
            throw new InvalidOperationException("OpenProcess failed");
        try
        {
            ProcessModule main = process.MainModule ?? throw new InvalidOperationException("Client image unavailable");
            int loaderRva = 0;
            int callerCallRva = FindInMemory(process, handle, main.BaseAddress, caller, callerMask, offset =>
            {
                int call = offset + 16;
                int displacement = BitConverter.ToInt32(Read(handle, main.BaseAddress + call + 1, 4));
                loaderRva = call + 5 + displacement;
                return loaderRva > 0;
            }) + 16;
            Console.WriteLine($"tact_caller_rva=0x{callerCallRva:X}");
            Console.WriteLine($"tact_loader_rva=0x{loaderRva:X}");
            int allocatorRva = FindInMemory(process, handle, main.BaseAddress, allocator, null, null);
            Console.WriteLine($"game_allocator_rva=0x{allocatorRva:X}");
            int copyRva = FindInMemory(process, handle, main.BaseAddress, copy, null, null);
            Console.WriteLine($"game_copy_rva=0x{copyRva:X}");
            int tagRva = FindInMemory(process, handle, main.BaseAddress, tag, null, null);
            Console.WriteLine($"allocator_tag_rva=0x{tagRva:X}");
        }
        finally
        {
            DiscoveryNative.CloseHandle(handle);
        }
    }

    private static byte[] Pattern(byte[] source, int offset, int length)
    {
        if (offset < 0 || length <= 0 || offset + length > source.Length)
            throw new InvalidDataException("Reference signature outside its decoded section");
        byte[] pattern = source.AsSpan(offset, length).ToArray();
        if (pattern.All(value => value == 0))
            throw new InvalidDataException("Empty reference signature");
        return pattern;
    }

    private static int FindInMemory(
        Process process,
        nint handle,
        nint mainBase,
        byte[] pattern,
        byte[]? mask,
        Func<int, bool>? predicate)
    {
        int found = -1;
        int regions = 0;
        long bytes = 0;
        long address = 0;
        while (address >= 0)
        {
            if (DiscoveryNative.VirtualQueryEx(
                    handle, (nint)address, out MemoryBasicInformation region, (nuint)48) == 0)
                break;
            long regionSize = (long)region.RegionSize;
            if (regionSize > 0 &&
                (region.State & 0x1000) != 0 &&
                (region.Protect & 0xFE) != 0 &&
                regionSize <= MaxImageBytes)
            {
                long offset64 = region.BaseAddress.ToInt64() - mainBase.ToInt64();
                if (offset64 >= int.MinValue && offset64 <= int.MaxValue)
                {
                    regions++;
                    bytes += regionSize;
                    byte[] image = Read(handle, region.BaseAddress, (int)regionSize);
                    int moduleOffset = (int)offset64;
                    Func<int, bool>? modulePredicate = predicate == null
                        ? null
                        : offset => predicate(checked(moduleOffset + offset));
                    int local = FindUnique(image, pattern, mask, modulePredicate, false);
                    CryptographicOperations.ZeroMemory(image);
                    if (local >= 0)
                    {
                        if (found >= 0)
                            throw new InvalidOperationException("Certificate discovery signature is ambiguous across memory regions");
                        found = checked(moduleOffset + local);
                    }
                }
            }
            long next = region.BaseAddress.ToInt64() + regionSize;
            if (next <= address)
                break;
            address = next;
        }
        if (found < 0)
        {
            Console.Error.WriteLine($"discovery scanned_regions={regions} bytes={bytes}");
            throw new InvalidOperationException("Certificate discovery signature was not found");
        }
        return found;
    }

    private static int FindUnique(
        byte[] image,
        byte[] pattern,
        byte[]? mask,
        Func<int, bool>? predicate,
        bool required = true)
    {
        int found = -1;
        int start = 0;
        while (start <= image.Length - pattern.Length)
        {
            int relative = image.AsSpan(start).IndexOf(pattern[0]);
            if (relative < 0)
                break;
            int offset = start + relative;
            bool matches = true;
            for (int index = 0; index < pattern.Length; index++)
            {
                if (mask != null && mask[index] == 0)
                    continue;
                if (image[offset + index] != pattern[index])
                {
                    matches = false;
                    break;
                }
            }
            if (matches && (predicate == null || predicate(offset)))
            {
                if (found >= 0)
                    throw new InvalidOperationException("Certificate discovery signature is ambiguous");
                found = offset;
            }
            start = offset + 1;
        }
        if (found < 0 && required)
            throw new InvalidOperationException("Certificate discovery signature was not found");
        return found;
    }

    private static byte[] Read(nint process, nint address, int count)
    {
        byte[] data = new byte[count];
        if (!DiscoveryNative.ReadProcessMemory(process, address, data, count, out nint read) || read != count)
            throw new InvalidOperationException("Bounded client image read failed");
        return data;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MemoryBasicInformation
    {
        internal nint BaseAddress;
        internal nint AllocationBase;
        internal uint AllocationProtect;
        internal ushort PartitionId;
        internal nuint RegionSize;
        internal uint State;
        internal uint Protect;
        internal uint Type;
    }

    private static class DiscoveryNative
    {
        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern nint OpenProcess(uint access, bool inherit, int id);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern nuint VirtualQueryEx(
            nint process,
            nint address,
            out MemoryBasicInformation information,
            nuint length);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool ReadProcessMemory(
            nint process, nint address, byte[] data, int size, out nint read);

        [DllImport("kernel32.dll")]
        internal static extern bool CloseHandle(nint handle);
    }
}
