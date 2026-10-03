using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;

// Scoped UI input only. No process memory hook, auto-submit or creation success.
internal static class CharacterInput
{
    public static void Enter(string executable, string firstName, string surname)
    {
        if (!Valid(firstName, false) || !Valid(surname, true))
            throw new ArgumentException("Fixture names must contain 2..12 ASCII letters (surname may be empty)");
        string isolated = LocalCertificateProvider.RequireIsolatedClient(executable);
        if (Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(isolated))) !=
            LocalCertificateProvider.ExpectedClientSha256)
            throw new InvalidOperationException("Unverified executable");
        using Process process = LocalCertificateProvider.FindClient(isolated);
        nint window = process.MainWindowHandle;
        if (window == 0 || !InputNative.GetClientRect(window, out InputNative.Rect rect))
            throw new InvalidOperationException("Isolated client rectangle unavailable");
        // Observed Win32 client rectangle for the isolated 1440x1052 X11
        // window: caption/borders are not part of these input coordinates.
        if (rect.Right != 1432 || rect.Bottom != 1018)
            throw new InvalidOperationException($"Unexpected isolated client dimensions: {rect.Right}x{rect.Bottom}");
        WriteField(window, 550, 56, firstName);
        WriteField(window, 890, 56, surname);
        Console.WriteLine("Fixture names entered in the isolated creation UI; not submitted or saved.");
    }

    private static bool Valid(string value, bool mayBeEmpty) =>
        mayBeEmpty && value.Length == 0 || value.Length is >= 2 and <= 12 &&
        value.All(character => character is >= 'A' and <= 'Z' or >= 'a' and <= 'z');

    private static void WriteField(nint window, int x, int y, string value)
    {
        nint point = x | (y << 16);
        Post(window, 0x200, 0, point);
        Post(window, 0x201, 1, point);
        Post(window, 0x202, 0, point);
        Thread.Sleep(100);
        Post(window, 0x100, 0x23, 1); // End before bounded clearing.
        Post(window, 0x101, 0x23, unchecked((nint)0xc0000001));
        for (int index = 0; index < 64; ++index) Post(window, 0x102, 8, 1);
        foreach (char character in value) Post(window, 0x102, character, 1);
    }

    private static void Post(nint window, uint message, nint key, nint value)
    {
        if (!InputNative.PostMessage(window, message, key, value))
            throw new InvalidOperationException("Isolated UI input could not be queued");
    }

    private static class InputNative
    {
        [StructLayout(LayoutKind.Sequential)]
        internal struct Rect { public int Left, Top, Right, Bottom; }
        [DllImport("user32.dll", SetLastError = true)]
        internal static extern bool PostMessage(nint window, uint message, nint key, nint value);
        [DllImport("user32.dll", SetLastError = true)]
        internal static extern bool GetClientRect(nint window, out Rect rect);
    }
}
