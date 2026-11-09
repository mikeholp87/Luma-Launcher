using System.IO;
using System.IO.Pipes;
using System.Security.Principal;

namespace Luma;

internal static class ResidentCommands
{
    internal static readonly string[] Supported = ["--show", "--hide", "--toggle", "--settings", "--refresh", "--background", "--quit", "--version", "--diagnostics", "--self-test", "--help"];
    private static string PipeName => "LumaLauncher.Native." + WindowsIdentity.GetCurrent().User!.Value + "." + System.Diagnostics.Process.GetCurrentProcess().SessionId;
    internal static string Parse(string[] args) => args.Length == 0 ? "--show" : args.Length == 1 && Supported.Contains(args[0]) ? args[0] : throw new ArgumentException("Use one of: " + string.Join(" | ", Supported));
    internal static bool Send(string command)
    {
        try
        {
            using var client = new NamedPipeClientStream(".", PipeName, PipeDirection.Out, PipeOptions.CurrentUserOnly);
            client.Connect(5000);
            using var writer = new StreamWriter(client) { AutoFlush = true };
            writer.WriteLine(command);
            return true;
        }
        catch (Exception ex) when (ex is IOException or TimeoutException or UnauthorizedAccessException) { Console.Error.WriteLine("Could not contact Luma: " + ex.Message); return false; }
    }
    internal static async Task Listen(Action<string> dispatch, CancellationToken stop)
    {
        while (!stop.IsCancellationRequested)
        {
            try
            {
                using var server = new NamedPipeServerStream(PipeName, PipeDirection.In, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);
                await server.WaitForConnectionAsync(stop);
                using var timeout = CancellationTokenSource.CreateLinkedTokenSource(stop);
                timeout.CancelAfter(TimeSpan.FromSeconds(2));
                // Bound input even if another local client never sends a newline.
                var buffer = new byte[64];
                int length = 0;
                while (length < buffer.Length)
                {
                    int read = await server.ReadAsync(buffer.AsMemory(length, 1), timeout.Token);
                    if (read == 0 || buffer[length] == '\n') break;
                    length += read;
                }
                string command = System.Text.Encoding.UTF8.GetString(buffer, 0, length).TrimEnd('\r');
                if (Supported.Contains(command)) dispatch(command);
            }
            catch (OperationCanceledException) { if (stop.IsCancellationRequested) break; }
            catch (IOException) { }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                // An unexpected error must not end the listener, or --toggle/--show would silently stop working.
                try { await Task.Delay(250, stop); } catch (OperationCanceledException) { break; }
            }
        }
    }
}
