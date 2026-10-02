using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Buffers.Binary;
using System.ComponentModel;

// Read-only metadata observation for the isolated 1.60.1.70170 client.
// This hook never changes certificate trust, the digest, or any signature path.
internal static class WorldCertificates
{
    private const string ExpectedClientSha256 =
        "369CE842043F6177850947274287FC5A6CEC3EE033A891FAA0400A1D0A475D8E";
    private const int ConstructorRva = 0x1F25730;
    private const int CaptureOffset = 0x200;
    private const int ReadyOffset = 0x220;
    private const int VectorOffset = 0x28;
    private const int CountOffset = 0x30;
    private const int ElementSize = 40;
    private const int PublicKeyOffset = 4;
    private const int FlagOffset = 36;
    private const uint ProcessAccess = 0x438; // query + VM read/write/operation
    private const uint MemCommit = 0x1000;
    private const uint MemReserve = 0x2000;
    private const uint PageExecuteReadWrite = 0x40;

    private static readonly byte[] ConstructorPrologue =
        Convert.FromHexString("40555341544156488D6C24C14881ECD8000000");

    private static readonly byte[] KnownWorldPublicKey =
        Convert.FromHexString("02596f0d0c061a8b30745988fd72c59e29ec367fb0f341f28e0f08d037bafc69");

    public static void Observe(string executable)
    {
        RequireIsolatedClient(executable);
        if (!File.Exists(executable))
            throw new FileNotFoundException("Isolated client not found", executable);

        string hash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(executable)));
        if (!hash.Equals(ExpectedClientSha256, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Unverified executable");

        using Process process = FindClient(executable);
        nint processHandle = WorldNative.OpenProcess(ProcessAccess, false, process.Id);
        if (processHandle == 0)
            throw new InvalidOperationException("OpenProcess failed");

        nint allocation = 0;
        bool installed = false;
        uint originalProtection = 0;
        nint entry = 0;

        try
        {
            nint imageBase = process.MainModule?.BaseAddress
                ?? throw new InvalidOperationException("Client image unavailable");
            entry = Add(imageBase, ConstructorRva);
            if (!Read(processHandle, entry, ConstructorPrologue.Length)
                    .SequenceEqual(ConstructorPrologue))
                throw new InvalidOperationException("Unexpected certificate constructor; no modification made");

            allocation = WorldNative.VirtualAllocEx(
                processHandle, 0, 0x1000, MemCommit | MemReserve, PageExecuteReadWrite);
            if (allocation == 0)
                throw new InvalidOperationException("Observation allocation failed");

            nint capturedObject = Add(allocation, CaptureOffset);
            nint readyFlag = Add(allocation, ReadyOffset);
            byte[] trampoline = BuildTrampoline(capturedObject, Add(entry, ConstructorPrologue.Length));
            Write(processHandle, allocation, trampoline);

            if (!WorldNative.VirtualProtectEx(
                    processHandle, entry, (nuint)ConstructorPrologue.Length,
                    PageExecuteReadWrite, out originalProtection))
                throw new InvalidOperationException("Cannot install observation");

            installed = true;
            byte[] patch = JumpWithPadding(allocation, ConstructorPrologue.Length);
            Write(processHandle, entry, patch);
            WorldNative.FlushInstructionCache(processHandle, allocation, (nuint)trampoline.Length);
            WorldNative.FlushInstructionCache(processHandle, entry, (nuint)patch.Length);

            Console.WriteLine("World certificate observation ready; perform one isolated world login.");
            WaitForReady(process, processHandle, readyFlag);
            Thread.Sleep(500);
            nint objectAddress = new(BinaryPrimitives.ReadInt64LittleEndian(Read(processHandle, capturedObject, 8)));
            if (objectAddress == 0) throw new InvalidOperationException("No constructor object observed");
            CertificateVector vector = ReadStableVector(processHandle, objectAddress);
            IReadOnlyList<CertificateRecord> records = ReadRecords(processHandle, vector);

            Console.WriteLine(
                $"world_certificates count={records.Count} " +
                $"region_ids={string.Join(',', records.Select(record => record.RegionId))} " +
                $"flags={string.Join(',', records.Select(record => $"0x{record.Flags:X2}"))} " +
                $"key_matches={string.Join(',', records.Select(record => record.KeyMatches ? "true" : "false"))}");
        }
        finally
        {
            try
            {
                if (installed && !HasExited(process))
                {
                    Write(processHandle, entry, ConstructorPrologue);
                    WorldNative.FlushInstructionCache(processHandle, entry, (nuint)ConstructorPrologue.Length);
                    if (!WorldNative.VirtualProtectEx(processHandle, entry, (nuint)ConstructorPrologue.Length,
                        originalProtection, out _)) throw new InvalidOperationException("Constructor protection restoration failed");
                    Console.WriteLine("Original certificate constructor restored.");
                }
            }
            finally
            {
                // Retain one page until exit: a client thread may still be in its tail.
                WorldNative.CloseHandle(processHandle);
            }
        }
    }

    private static Process FindClient(string executable)
    {
        Process[] matches = Process.GetProcessesByName("WowB")
            .Where(process =>
            {
                try
                {
                    return string.Equals(
                        Path.GetFullPath(process.MainModule?.FileName ?? string.Empty),
                        executable, StringComparison.OrdinalIgnoreCase);
                }
                catch (InvalidOperationException)
                {
                    return false;
                }
                catch (Win32Exception)
                {
                    return false;
                }
            })
            .ToArray();

        if (matches.Length != 1)
            throw new InvalidOperationException("Expected exactly one matching isolated WowB process");

        return matches[0];
    }

    private static void RequireIsolatedClient(string executable)
    {
        string full = Path.GetFullPath(executable);
        string normalized = full.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        string marker = $"{Path.DirectorySeparatorChar}target{Path.DirectorySeparatorChar}forever-login"
            + $"{Path.DirectorySeparatorChar}client{Path.DirectorySeparatorChar}";
        if (!normalized.Contains(marker, StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetFileName(normalized), "WowB.exe", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Only the isolated Forever WowB.exe is allowed");
    }

    private static void WaitForReady(Process process, nint processHandle, nint readyFlag)
    {
        DateTime deadline = DateTime.UtcNow.AddSeconds(90);
        while (DateTime.UtcNow < deadline && !HasExited(process))
        {
            if (BinaryPrimitives.ReadUInt32LittleEndian(Read(processHandle, readyFlag, sizeof(uint))) == 1)
                return;
            Thread.Sleep(100);
        }

        throw new TimeoutException("No world certificate metadata observation occurred");
    }

    private static CertificateVector ReadStableVector(nint processHandle, nint objectAddress)
    {
        CertificateVector first = ReadVector(processHandle, objectAddress);
        if (first.Count == 0 || first.Count > 32)
            throw new InvalidOperationException("World certificate count outside bounded range");

        Thread.Sleep(50);
        CertificateVector second = ReadVector(processHandle, objectAddress);
        if (first.Pointer != second.Pointer || first.Count != second.Count)
            throw new InvalidOperationException("World certificate vector is not stable");

        return second;
    }

    private static CertificateVector ReadVector(nint processHandle, nint objectAddress)
    {
        byte[] fields = Read(processHandle, Add(objectAddress, VectorOffset), 16);
        nint pointer = new(BinaryPrimitives.ReadInt64LittleEndian(fields.AsSpan(0, 8)));
        ulong count = BinaryPrimitives.ReadUInt64LittleEndian(fields.AsSpan(CountOffset - VectorOffset, 8));
        return new CertificateVector(pointer, count);
    }

    private static IReadOnlyList<CertificateRecord> ReadRecords(
        nint processHandle, CertificateVector vector)
    {
        var records = new List<CertificateRecord>(checked((int)vector.Count));
        for (ulong index = 0; index < vector.Count; index++)
        {
            nint address = Add(vector.Pointer, checked((int)(index * (ulong)ElementSize)));
            byte[] record = Read(processHandle, address, ElementSize);
            int regionId = BinaryPrimitives.ReadInt32LittleEndian(record.AsSpan(0, sizeof(int)));
            byte flags = record[FlagOffset];
            bool keyMatches = CryptographicOperations.FixedTimeEquals(
                record.AsSpan(PublicKeyOffset, KnownWorldPublicKey.Length), KnownWorldPublicKey);
            records.Add(new CertificateRecord(regionId, flags, keyMatches));
            CryptographicOperations.ZeroMemory(record);
        }

        return records;
    }

    private static byte[] Read(nint processHandle, nint address, int size)
    {
        byte[] data = new byte[size];
        if (!WorldNative.ReadProcessMemory(processHandle, address, data, size, out nint read) || read != size)
            throw new InvalidOperationException("Bounded metadata read failed");
        return data;
    }

    private static void Write(nint processHandle, nint address, byte[] data)
    {
        if (!WorldNative.WriteProcessMemory(processHandle, address, data, data.Length, out nint written) ||
            written != data.Length)
            throw new InvalidOperationException("Bounded observation write failed");
    }

    private static byte[] BuildTrampoline(nint capturedObject, nint returnAddress)
    {
        var code = new List<byte>();
        code.Add(0x50); // push rax
        code.Add(0x52); // push rdx
        code.AddRange(MoveRax(capturedObject));
        code.AddRange(new byte[] { 0x48, 0x89, 0x08 }); // mov [rax], rcx
        code.AddRange(new byte[] { 0xC7, 0x40, 0x20, 0x01, 0x00, 0x00, 0x00 });
        code.Add(0x5A); // pop rdx
        code.Add(0x58); // pop rax
        code.AddRange(ConstructorPrologue);
        code.AddRange(Jump(returnAddress));
        return code.ToArray();
    }

    private static byte[] MoveRax(nint value) => new byte[] { 0x48, 0xB8 }
        .Concat(BitConverter.GetBytes(value.ToInt64())).ToArray();

    private static byte[] Jump(nint target) => new byte[] { 0xFF, 0x25, 0, 0, 0, 0 }
        .Concat(BitConverter.GetBytes(target.ToInt64())).ToArray();

    private static byte[] JumpWithPadding(nint target, int length)
    {
        byte[] patch = Enumerable.Repeat((byte)0x90, length).ToArray();
        Jump(target).CopyTo(patch, 0);
        return patch;
    }

    private static nint Add(nint address, int offset) =>
        new(checked(address.ToInt64() + offset));

    private static bool HasExited(Process process)
    {
        try
        {
            return process.HasExited;
        }
        catch (InvalidOperationException)
        {
            return true;
        }
    }

    private readonly record struct CertificateVector(nint Pointer, ulong Count);
    private readonly record struct CertificateRecord(int RegionId, byte Flags, bool KeyMatches);

    private static class WorldNative
    {
        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern nint OpenProcess(uint access, bool inherit, int id);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool ReadProcessMemory(
            nint process, nint address, byte[] data, int size, out nint read);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool WriteProcessMemory(
            nint process, nint address, byte[] data, int size, out nint written);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern nint VirtualAllocEx(
            nint process, nint address, nuint size, uint allocationType, uint protection);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool VirtualProtectEx(
            nint process, nint address, nuint size, uint protection, out uint oldProtection);

        [DllImport("kernel32.dll")]
        internal static extern bool FlushInstructionCache(nint process, nint address, nuint size);

        [DllImport("kernel32.dll")]
        internal static extern bool CloseHandle(nint handle);
    }
}
