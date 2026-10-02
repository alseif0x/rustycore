// Operator-only observation of the isolated, hash-pinned 70170 client.
// The digest implementation and its arguments/results remain unchanged.
// Source anchors: caller 0x1F2C0E1 -> digest 0x4721110; sixth Win64 argument
// is the 16-byte build key obtained through the client's own protected getter.
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;

if (args.Length != 3 || args[0] != "--capture")
    throw new ArgumentException("Usage: --capture <isolated WowB.exe> <new private key file>");
string executable = Path.GetFullPath(args[1]);
string output = Path.GetFullPath(args[2]);
if (!executable.Contains(@"\target\forever-login\client\", StringComparison.OrdinalIgnoreCase) ||
    !output.Contains(@"\target\forever-login\", StringComparison.OrdinalIgnoreCase))
    throw new ArgumentException("Only the isolated Forever fixture is allowed");
if (File.Exists(output)) throw new IOException("Refusing to overwrite a key file");
if (Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(executable))) !=
    "369CE842043F6177850947274287FC5A6CEC3EE033A891FAA0400A1D0A475D8E")
    throw new InvalidOperationException("Unverified executable");
var process = Process.GetProcessesByName("WowB").Single(p =>
    string.Equals(Path.GetFullPath(p.MainModule!.FileName), executable, StringComparison.OrdinalIgnoreCase));
nint handle = Native.OpenProcess(0x438, false, process.Id);
if (handle == 0) throw new InvalidOperationException("OpenProcess failed");
byte[] original = Convert.FromHexString("48895C2408488974241048897C2418");
nint entry = process.MainModule!.BaseAddress + 0x4721110;
bool installed = false;
uint protection = 0;
byte[] key = new byte[16];
try
{
    if (!Read(entry, original.Length).SequenceEqual(original))
        throw new InvalidOperationException("Unexpected digest entry; no modification made");
    nint allocation = Native.VirtualAllocEx(handle, 0, 4096, 0x3000, 0x40);
    if (allocation == 0) throw new InvalidOperationException("Observation allocation failed");
    nint captured = allocation + 0x200;
    // Save volatile registers, copy the sixth original argument when non-null,
    // restore registers, execute the exact replaced instructions and resume.
    var code = new List<byte>();
    code.AddRange(Convert.FromHexString("5052488B44244048BA"));
    code.AddRange(BitConverter.GetBytes((long)captured));
    code.AddRange(Convert.FromHexString("4885C0740D0F10000F1102C74210010000005A58"));
    code.AddRange(original);
    code.AddRange(Jump(entry + original.Length));
    Write(allocation, code.ToArray());
    if (!Native.VirtualProtectEx(handle, entry, (nuint)original.Length, 0x40, out protection))
        throw new InvalidOperationException("Cannot install observation");
    installed = true;
    Write(entry, Jump(allocation).Concat(new byte[] {0x90}).ToArray());
    Native.FlushInstructionCache(handle, allocation, (nuint)code.Count);
    Native.FlushInstructionCache(handle, entry, (nuint)original.Length);
    Console.WriteLine("Build-key observation ready; perform one isolated world login.");
    var deadline = DateTime.UtcNow.AddSeconds(90);
    bool ready = false;
    while (DateTime.UtcNow < deadline && !process.HasExited)
    {
        if (BitConverter.ToUInt32(Read(captured + 16, 4)) == 1) { ready = true; break; }
        Thread.Sleep(100);
    }
    if (!ready) throw new TimeoutException("No build-key calculation observed");
    key = Read(captured, 16);
    if (key.All(value => value == 0)) throw new InvalidOperationException("Observed zero build key");
    using (var file = new FileStream(output, FileMode.CreateNew, FileAccess.Write, FileShare.None))
        file.Write(key);
    Write(captured, new byte[20]);
    Console.WriteLine("Build key captured privately (16 bytes); digest calculation preserved.");
    // Retain this one page until client exit: the original thread may still be
    // finishing the trampoline when the observer sees its ready flag.
}
finally
{
    CryptographicOperations.ZeroMemory(key);
    if (installed && !process.HasExited)
    {
        Write(entry, original);
        Native.FlushInstructionCache(handle, entry, (nuint)original.Length);
        Native.VirtualProtectEx(handle, entry, (nuint)original.Length, protection, out _);
        Console.WriteLine("Original digest entry restored.");
    }
    Native.CloseHandle(handle);
}

byte[] Read(nint address, int count)
{
    byte[] data = new byte[count];
    if (!Native.ReadProcessMemory(handle, address, data, count, out nint read) || read != count)
        throw new InvalidOperationException("Bounded observation read failed");
    return data;
}
void Write(nint address, byte[] data)
{
    if (!Native.WriteProcessMemory(handle, address, data, data.Length, out nint written) || written != data.Length)
        throw new InvalidOperationException("Bounded observation write failed");
}
static byte[] Jump(nint target) => Convert.FromHexString("FF2500000000")
    .Concat(BitConverter.GetBytes((long)target)).ToArray();

static class Native
{
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern nint OpenProcess(uint access, bool inherit, int id);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool ReadProcessMemory(nint process, nint address, byte[] data, int size, out nint read);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool WriteProcessMemory(nint process, nint address, byte[] data, int size, out nint written);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern nint VirtualAllocEx(nint process, nint address, nuint size, uint type, uint protection);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool VirtualProtectEx(nint process, nint address, nuint size, uint protection, out uint old);
    [DllImport("kernel32.dll")]
    public static extern bool FlushInstructionCache(nint process, nint address, nuint size);
    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(nint handle);
}
