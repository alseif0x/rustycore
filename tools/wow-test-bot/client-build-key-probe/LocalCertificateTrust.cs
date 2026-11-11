using System.Security.Cryptography.X509Certificates;

internal static class LocalCertificateTrust
{
    public static void Install(string pemPath)
    {
        string full = Path.GetFullPath(pemPath).Replace('/', '\\');
        if (!full.Contains("\\target\\forever-login\\bnetserver.cert.pem", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Only the isolated public BNet certificate is allowed");
        using var certificate = new X509Certificate2(full);
        using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
        store.Open(OpenFlags.ReadWrite);
        store.Add(certificate);
        store.Close();
        using var verify = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
        verify.Open(OpenFlags.ReadOnly);
        if (!verify.Certificates.Contains(certificate))
            throw new InvalidOperationException("Certificate was not persisted in the isolated root store");
        Console.WriteLine("Isolated root certificate persisted.");
    }
}
