using System.IO;
using System.Runtime.InteropServices;

namespace Luma;

internal static class ShellMetadata
{
    internal static Entry Read(string path, string root)
    {
        string description = "", target = "";
        object? shell = null, link = null;
        try
        {
            var type = Type.GetTypeFromProgID("WScript.Shell");
            if (type is not null)
            {
                shell = Activator.CreateInstance(type);
                link = ((dynamic)shell!).CreateShortcut(path);
                description = ((dynamic)link).Description as string ?? "";
                target = ((dynamic)link).TargetPath as string ?? "";
            }
        }
        catch (Exception) { /* A broken shortcut must not discard the rest of the index. */ }
        finally
        {
            if (link is not null && Marshal.IsComObject(link)) Marshal.FinalReleaseComObject(link);
            if (shell is not null && Marshal.IsComObject(shell)) Marshal.FinalReleaseComObject(shell);
        }
        string category = Path.GetDirectoryName(Path.GetRelativePath(root, path)) ?? "";
        return new Entry(Path.GetFileNameWithoutExtension(path), path, description, category + " " + Path.GetFileNameWithoutExtension(target));
    }
}
