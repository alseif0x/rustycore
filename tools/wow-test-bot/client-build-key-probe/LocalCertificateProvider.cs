using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;

// Operator-only provider for the isolated 1.60.1.70170 client. It replaces
// only the native TACT certificate-resource loader long enough for the native
// PEM/X.509 parser and signature path to run unchanged.
internal static class LocalCertificateProvider
{
    private const string ExpectedClientSha256 =
        "369CE842043F6177850947274287FC5A6CEC3EE033A891FAA0400A1D0A475D8E";
    private const int TactLoaderRva = 0x4721670;
    private const int TactFileIdCallRva = 0x472169B;
    private const int TactCallerRva = 0x1F25778;
    private const int GameAllocatorRva = 0x46027E0;
    private const int GameCopyRva = 0x483D490;
    private const int AllocatorTagRva = 0x4CFD090;
    private const int MaxPemBytes = 64 * 1024;
    private const uint ProcessAccess =
        0x0400u | // PROCESS_QUERY_INFORMATION
        0x0008u | // PROCESS_VM_OPERATION
        0x0010u | // PROCESS_VM_READ
        0x0020u | // PROCESS_VM_WRITE
        0x0800u;  // PROCESS_SUSPEND_RESUME
    private const uint MemCommit = 0x1000;
    private const uint MemReserve = 0x2000;
    private const uint PageReadWrite = 0x04;
    private const uint PageReadOnly = 0x02;
    private const uint PageExecuteRead = 0x20;
    private const uint PageExecuteReadWrite = 0x40;

    private static readonly byte[] ExpectedLoaderPrologue =
        Convert.FromHexString("4C8BDC53564881EC88000000");
    private static readonly byte[] ExpectedFileIdCall =
        Convert.FromHexString("41B9DAE17500");
    private static readonly byte[] ExpectedCallerCall =
        Convert.FromHexString("E8F3BE7F02");

    public static void Install(string executable, string pemPath)
    {
        string isolatedExecutable = RequireIsolatedClient(executable);
        byte[] pem = ReadCertificateBundle(pemPath);
        try
        {
            string hash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(isolatedExecutable)));
            if (!hash.Equals(ExpectedClientSha256, StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("Unverified executable");

            using Process process = FindClient(isolatedExecutable);
            nint processHandle = LocalNative.OpenProcess(ProcessAccess, false, process.Id);
            if (processHandle == 0)
                throw new InvalidOperationException("OpenProcess failed");

            bool suspended = false;
            bool installationComplete = false;
            bool patchAttempted = false;
            uint originalProtection = 0;
            nint entry = 0;
            nint data = 0;
            nint code = 0;
            nint counter = 0;
            try
            {
                nint imageBase = process.MainModule?.BaseAddress
                    ?? throw new InvalidOperationException("Client image unavailable");
                entry = Add(imageBase, TactLoaderRva);
                VerifyProvenance(processHandle, imageBase);

                if (LocalNative.NtSuspendProcess(processHandle) != 0)
                    throw new InvalidOperationException("Cannot pause isolated client");
                suspended = true;

                // Recheck after suspension so no code is written after a stale
                // pre-suspend provenance observation.
                VerifyProvenance(processHandle, imageBase);

                data = LocalNative.VirtualAllocEx(
                    processHandle, 0, checked((nuint)(pem.Length + 1)),
                    MemCommit | MemReserve, PageReadWrite);
                code = LocalNative.VirtualAllocEx(
                    processHandle, 0, 0x1000, MemCommit | MemReserve, PageReadWrite);
                // Keep diagnostics writable without making code or PEM writable.
                counter = LocalNative.VirtualAllocEx(
                    processHandle, 0, 0x1000, MemCommit | MemReserve, PageReadWrite);
                if (data == 0 || code == 0 || counter == 0)
                    throw new InvalidOperationException("Provider allocation failed");

                byte[] stub = BuildProviderStub(imageBase, data, pem.Length, counter);
                Write(processHandle, data, pem.Concat(new byte[] { 0 }).ToArray());
                Write(processHandle, code, stub);
                if (!LocalNative.VirtualProtectEx(
                        processHandle, data, checked((nuint)(pem.Length + 1)),
                        PageReadOnly, out _))
                    throw new InvalidOperationException("Certificate data protection failed");
                if (!LocalNative.VirtualProtectEx(
                        processHandle, code, (nuint)stub.Length,
                        PageExecuteRead, out _))
                    throw new InvalidOperationException("Provider code protection failed");

                byte[] original = Read(processHandle, entry, ExpectedLoaderPrologue.Length);
                if (!original.SequenceEqual(ExpectedLoaderPrologue))
                    throw new InvalidOperationException("Unexpected certificate loader; no modification made");
                if (!LocalNative.VirtualProtectEx(
                        processHandle, entry, (nuint)ExpectedLoaderPrologue.Length,
                        PageExecuteReadWrite, out originalProtection))
                    throw new InvalidOperationException("Cannot protect certificate loader");

                patchAttempted = true;
                Write(processHandle, entry, Jump(code));
                Flush(processHandle, code, (nuint)stub.Length);
                Flush(processHandle, entry, (nuint)ExpectedLoaderPrologue.Length);
                if (!LocalNative.VirtualProtectEx(
                        processHandle, entry, (nuint)ExpectedLoaderPrologue.Length,
                        originalProtection, out _))
                    throw new InvalidOperationException("Certificate loader protection restoration failed");

                installationComplete = true;
                Console.WriteLine("Installed local TACT 7725530 certificate provider.");
                Console.WriteLine($"Provider call counter address: {counter.ToInt64():X}");
            }
            finally
            {
                try
                {
                    if (patchAttempted && !installationComplete)
                        RestoreLoader(processHandle, entry, originalProtection);
                }
                finally
                {
                    try
                    {
                        if (suspended)
                        {
                            int status = LocalNative.NtResumeProcess(processHandle);
                            if (status != 0)
                                throw new InvalidOperationException("Cannot resume isolated client");
                        }
                    }
                    finally
                    {
                        LocalNative.CloseHandle(processHandle);
                    }
                }
            }
        }
        finally
        {
            CryptographicOperations.ZeroMemory(pem);
        }
    }

    private static string RequireIsolatedClient(string executable)
    {
        string full = Path.GetFullPath(executable);
        string normalized = full.Replace('/', '\\');
        string marker = "\\target\\forever-login\\client\\";
        if (!normalized.Contains(marker, StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetFileName(full), "WowB.exe", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Only the isolated Forever WowB.exe is allowed");
        return full;
    }

    private static byte[] ReadCertificateBundle(string path)
    {
        string full = Path.GetFullPath(path);
        FileInfo info = new(full);
        if (!info.Exists)
            throw new FileNotFoundException("Certificate bundle not found", full);
        if (info.Length == 0 || info.Length > MaxPemBytes)
            throw new InvalidDataException("Certificate bundle must be 1..65536 bytes");

        byte[] bytes = File.ReadAllBytes(full);
        if (bytes.Length > MaxPemBytes || bytes.Any(value => value > 0x7f))
            throw new InvalidDataException("Certificate bundle must be ASCII PEM");
        string text = Encoding.ASCII.GetString(bytes);
        if (text.Contains("PRIVATE KEY", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Private keys are not accepted");

        const string begin = "-----BEGIN CERTIFICATE-----";
        const string end = "-----END CERTIFICATE-----";
        int offset = 0;
        int certificateCount = 0;
        while (true)
        {
            while (offset < text.Length && char.IsWhiteSpace(text[offset]))
                offset++;
            if (offset == text.Length)
                break;
            if (!text.AsSpan(offset).StartsWith(begin, StringComparison.Ordinal))
                throw new InvalidDataException("Certificate bundle contains a non-certificate PEM block");

            int bodyStart = offset + begin.Length;
            int endStart = text.IndexOf(end, bodyStart, StringComparison.Ordinal);
            if (endStart < 0)
                throw new InvalidDataException("Certificate PEM block is incomplete");
            string encoded = RemoveWhitespace(text[bodyStart..endStart]);
            byte[] der;
            try
            {
                der = Convert.FromBase64String(encoded);
                using X509Certificate2 certificate = new(der);
                if (certificate.HasPrivateKey)
                    throw new InvalidDataException("Private keys are not accepted");
            }
            catch (FormatException exception)
            {
                throw new InvalidDataException("Certificate PEM block is not valid base64", exception);
            }
            catch (CryptographicException exception)
            {
                throw new InvalidDataException("Certificate PEM block is not an X.509 certificate", exception);
            }

            certificateCount++;
            offset = endStart + end.Length;
        }

        if (certificateCount == 0)
            throw new InvalidDataException("Certificate bundle contains no certificates");
        return bytes;
    }

    private static string RemoveWhitespace(string value)
    {
        var builder = new StringBuilder(value.Length);
        foreach (char character in value)
        {
            if (!char.IsWhiteSpace(character))
                builder.Append(character);
        }
        return builder.ToString();
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

    private static void VerifyProvenance(nint processHandle, nint imageBase)
    {
        if (!Read(processHandle, Add(imageBase, TactLoaderRva), ExpectedLoaderPrologue.Length)
                .SequenceEqual(ExpectedLoaderPrologue) ||
            !Read(processHandle, Add(imageBase, TactFileIdCallRva), ExpectedFileIdCall.Length)
                .SequenceEqual(ExpectedFileIdCall) ||
            !Read(processHandle, Add(imageBase, TactCallerRva), ExpectedCallerCall.Length)
                .SequenceEqual(ExpectedCallerCall))
            throw new InvalidOperationException("Decoded certificate provider provenance mismatch");
    }

    private static void RestoreLoader(nint processHandle, nint entry, uint originalProtection)
    {
        if (!LocalNative.VirtualProtectEx(
                processHandle, entry, (nuint)ExpectedLoaderPrologue.Length,
                PageExecuteReadWrite, out _))
            throw new InvalidOperationException("Cannot protect certificate loader for restoration");
        try
        {
            Write(processHandle, entry, ExpectedLoaderPrologue);
            Flush(processHandle, entry, (nuint)ExpectedLoaderPrologue.Length);
        }
        finally
        {
            if (!LocalNative.VirtualProtectEx(
                    processHandle, entry, (nuint)ExpectedLoaderPrologue.Length,
                    originalProtection, out _))
                throw new InvalidOperationException("Certificate loader protection restoration failed");
        }
    }

    private static byte[] BuildProviderStub(nint imageBase, nint data, int pemLength, nint counter)
    {
        var code = new List<byte>();
        Emit(code, "48B8");
        U64(code, (ulong)counter);
        Emit(code, "F048FF00"); // lock inc [rax]; RAX/flags are call-volatile
        Emit(code, "534883EC30488BD9"); // push rbx; shadow space; rbx=sret
        Emit(code, "B9");
        U32(code, checked((uint)pemLength + 1));
        Emit(code, "BA09000000");
        Emit(code, "49B8");
        U64(code, (ulong)Add(imageBase, AllocatorTagRva));
        Emit(code, "41B9FAFFFFFFC744242000000000");
        Emit(code, "48B8");
        U64(code, (ulong)Add(imageBase, GameAllocatorRva));
        Emit(code, "FFD048890348C74308");
        U32(code, checked((uint)pemLength));
        Emit(code, "48C74310");
        U32(code, checked((uint)pemLength));
        Emit(code, "48B9");
        U64(code, (ulong)data);
        Emit(code, "488BD1488BC841B8");
        U32(code, checked((uint)pemLength + 1));
        Emit(code, "48B8");
        U64(code, (ulong)Add(imageBase, GameCopyRva));
        Emit(code, "FFD0C6432801488BC34883C4305BC3");
        return code.ToArray();
    }

    private static void Emit(List<byte> code, string hex) => code.AddRange(Convert.FromHexString(hex));

    private static void U32(List<byte> code, uint value) => code.AddRange(BitConverter.GetBytes(value));

    private static void U64(List<byte> code, ulong value) => code.AddRange(BitConverter.GetBytes(value));

    private static byte[] Jump(nint target) => Convert.FromHexString("48B8")
        .Concat(BitConverter.GetBytes(target.ToInt64()))
        .Concat(Convert.FromHexString("FFE0"))
        .ToArray();

    private static byte[] Read(nint processHandle, nint address, int count)
    {
        byte[] data = new byte[count];
        if (!LocalNative.ReadProcessMemory(processHandle, address, data, count, out nint read) || read != count)
            throw new InvalidOperationException("Bounded client memory read failed");
        return data;
    }

    private static void Write(nint processHandle, nint address, byte[] data)
    {
        if (!LocalNative.WriteProcessMemory(processHandle, address, data, data.Length, out nint written) ||
            written != data.Length)
            throw new InvalidOperationException("Bounded client memory write failed");
    }

    private static void Flush(nint processHandle, nint address, nuint size)
    {
        if (!LocalNative.FlushInstructionCache(processHandle, address, size))
            throw new InvalidOperationException("Instruction-cache flush failed");
    }

    private static nint Add(nint address, int offset) =>
        new(checked(address.ToInt64() + offset));

    private static class LocalNative
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

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool FlushInstructionCache(nint process, nint address, nuint size);

        [DllImport("kernel32.dll", SetLastError = true)]
        internal static extern bool CloseHandle(nint handle);

        [DllImport("ntdll.dll")]
        internal static extern int NtSuspendProcess(nint process);

        [DllImport("ntdll.dll")]
        internal static extern int NtResumeProcess(nint process);
    }
}
