using System.Threading;
using System.Windows;
using System.Windows.Threading;

namespace Luma;

public partial class App : Application
{
    private Mutex? mutex;
    private bool ownsMutex;
    private readonly CancellationTokenSource residentStop = new();

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        string command;
        try { command = ResidentCommands.Parse(e.Args); }
        catch (ArgumentException ex) { Console.Error.WriteLine(ex.Message); Shutdown(2); return; }
        if (command == "--help") { Console.WriteLine("LumaLauncher " + string.Join(" | ", ResidentCommands.Supported)); Shutdown(); return; }
        if (command == "--version") { Console.WriteLine("Luma Launcher 0.1.5 (Windows)"); Shutdown(); return; }
        if (command == "--diagnostics")
        {
            Console.WriteLine($"Luma Launcher 0.1.5 (Windows)\nOS: {Environment.OSVersion}\n.NET: {Environment.Version}\nExecutable: {Environment.ProcessPath}\nSettings: {Core.DataPath}\nApplications: {Core.Discover().Count()}\nUsage source: Luma launch history\nShortcuts: Windows RegisterHotKey");
            Shutdown(); return;
        }
        if (command == "--self-test")
        {
            ShutdownMode = ShutdownMode.OnExplicitShutdown;
            Shutdown(Core.SelfTest() ? 0 : 1);
            return;
        }
        mutex = new Mutex(true, @"Local\LumaLauncher.Native", out bool created);
        if (!created)
        {
            Shutdown(ResidentCommands.Send(command) ? 0 : 1);
            return;
        }
        ownsMutex = true;
        if (command == "--quit") { mutex.ReleaseMutex(); mutex.Dispose(); mutex = null; ownsMutex = false; Shutdown(); return; }
        var window = new MainWindow();
        MainWindow = window;
        window.InitializeResident();
        _ = Task.Run(() => ResidentCommands.Listen(cmd => Dispatcher.BeginInvoke(new Action(() => window.Command(cmd))), residentStop.Token));
        window.Command(command);
    }

    protected override void OnExit(ExitEventArgs e)
    {
        residentStop.Cancel();
        // Only the owner may release the mutex. Application.MainWindow is cleared when the window closes, so track ownership explicitly.
        if (ownsMutex) mutex?.ReleaseMutex();
        mutex?.Dispose();
        base.OnExit(e);
    }
}
