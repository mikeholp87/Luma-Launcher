using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;

namespace Luma;

internal static class SettingsChecks
{
    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private static IEnumerable<T> Controls<T>(DependencyObject parent) where T : DependencyObject
    {
        foreach (var child in LogicalTreeHelper.GetChildren(parent).OfType<DependencyObject>())
        {
            if (child is T control) yield return control;
            foreach (var descendant in Controls<T>(child)) yield return descendant;
        }
    }

    internal static void Run()
    {
        string dir = Path.Combine(Path.GetTempPath(), "luma-settings-" + Guid.NewGuid().ToString("N"));
        try
        {
            string path = Path.Combine(dir, "settings.json");
            var original = new Settings();
            original.Favorites.Add("App.LNK");
            original.Usage["App.LNK"] = new Usage { Count = 12, Last = 34 };
            var edited = Core.Copy(original);
            edited.Theme = "Light"; edited.Accent = "#123ABC"; edited.Width = 900;
            edited.Results = 12; edited.FontSize = 24; edited.Compact = true;
            edited.ShowSettingsButton = false; edited.History = false; edited.Descriptions = false;
            edited.Fuzzy = false; edited.ClearOnShow = false; edited.StartAtLogin = true;
            edited.Hotkey = "Ctrl+Shift+F8"; edited.SettingsHotkey = "Ctrl+Shift+F9";
            Core.Save(path, edited);
            var loaded = Core.Load(path);
            Check(JsonSerializer.Serialize(edited) == JsonSerializer.Serialize(loaded), "Every setting must round-trip.");
            Check(loaded.Favorites.Contains("app.lnk") && loaded.Usage["app.lnk"].Count == 12, "Loaded paths must ignore case.");
            edited.Favorites.Clear(); edited.Usage["app.lnk"].Count = 99;
            Check(original.Favorites.Count == 1 && original.Usage["app.lnk"].Count == 12, "Edits must be isolated until saved.");
            string before = File.ReadAllText(path);
            loaded.Width = 10;
            try { Core.Save(path, loaded); throw new Exception("Invalid settings were saved."); } catch (InvalidDataException) { }
            Check(File.ReadAllText(path) == before, "Validation failure must preserve the saved file.");
            loaded.Width = 900;
            using (var locked = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.None))
            {
                try { Core.Save(path, loaded); throw new Exception("Locked settings were overwritten."); }
                catch (IOException) { }
                catch (UnauthorizedAccessException) { }
            }
            Check(File.ReadAllText(path) == before && !Directory.EnumerateFiles(dir, "*.tmp").Any(), "Failed writes must preserve the file and clean up temporary files.");
            loaded.Favorites.Add(new string('x', 4 * 1024 * 1024));
            try { Core.Save(path, loaded); throw new Exception("Oversized settings were saved."); } catch (InvalidDataException) { }
            Check(File.ReadAllText(path) == before, "Oversized writes must preserve the saved file.");

            int attempts = 0;
            var dialog = new SettingsWindow(original, candidate =>
            {
                if (++attempts == 1) throw new IOException("Simulated write failure");
                Core.Save(path, candidate);
            });
            Exception? failure = null;
            dialog.Dispatcher.BeginInvoke(new Action(() =>
            {
                try
                {
                    var width = Controls<TextBox>(dialog).First();
                    var save = Controls<Button>(dialog).Single(b => Equals(b.Content, "Save settings"));
                    width.Text = "invalid";
                    save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Check(dialog.IsVisible && attempts == 0, "Invalid input must keep the form open without saving.");
                    width.Text = "880";
                    save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                    Check(dialog.IsVisible && attempts == 1 && width.Text == "880", "Save failure must retain the form and edits.");
                    Check(Controls<TextBlock>(dialog).Any(t => t.Text == "Simulated write failure"), "Save error must be visible.");
                    save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                }
                catch (Exception ex) { failure = ex; dialog.DialogResult = false; }
            }));
            Check(dialog.ShowDialog() == true && failure is null && attempts == 2 && Core.Load(path).Width == 880, "Retry must save and close successfully: " + failure);

            var cancel = new SettingsWindow(original, _ => throw new Exception("Cancel must not save."));
            cancel.Dispatcher.BeginInvoke(new Action(() =>
            {
                Controls<Button>(cancel).Single(b => Equals(b.Content, "Reset personalization")).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Controls<Button>(cancel).Single(b => Equals(b.Content, "Discard pending edits")).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            }));
            Check(cancel.ShowDialog() == false && original.Favorites.Count == 1 && original.Usage.Count == 1 && Core.Load(path).Width == 880, "Cancel must preserve settings and personalization.");

            bool reject = false;
            var automatic = new SettingsWindow(original, candidate => { if (reject) throw new IOException("Simulated autosave failure"); Core.Save(path, candidate); });
            automatic.Show();
            try
            {
                var fields = Controls<TextBox>(automatic).ToArray();
                fields[0].Text = "920";
                fields[0].RaiseEvent(new RoutedEventArgs(UIElement.LostFocusEvent));
                Check(Core.Load(path).Width == 920, "Leaving a text field must autosave.");
                fields[2].Text = "25";
                fields[2].RaiseEvent(new RoutedEventArgs(UIElement.LostFocusEvent));
                Check(Core.Load(path).FontSize == 25, "Font size must autosave.");
                foreach (var check in Controls<CheckBox>(automatic))
                {
                    check.IsChecked = !check.IsChecked;
                    check.RaiseEvent(new RoutedEventArgs(CheckBox.ClickEvent));
                }
                var onDisk = Core.Load(path);
                Check(onDisk.Compact && !onDisk.ShowSettingsButton && !onDisk.History && !onDisk.Descriptions && !onDisk.Fuzzy && !onDisk.ClearOnShow && onDisk.StartAtLogin, "Every checkbox must autosave.");
                Controls<ComboBox>(automatic).Single().SelectedItem = "Light";
                Check(Core.Load(path).Theme == "Light", "Theme must autosave.");
                var clear = Controls<Button>(automatic).Single(b => Equals(b.Content, "Clear launch history"));
                clear.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Check(Core.Load(path).Usage.Count == 1, "The first history-clear click must only request confirmation.");
                clear.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Check(Core.Load(path).Usage.Count == 0 && Core.Load(path).Favorites.Count == 1, "History clearing must preserve favorites.");
                var reset = Controls<Button>(automatic).Single(b => Equals(b.Content, "Reset personalization"));
                reset.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Check(Core.Load(path).Favorites.Count == 1, "The first reset click must only request confirmation.");
                reset.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Check(Core.Load(path).Favorites.Count == 0, "Confirmed reset must persist.");
                reject = true;
                var compact = Controls<CheckBox>(automatic).First();
                compact.IsChecked = false; compact.RaiseEvent(new RoutedEventArgs(CheckBox.ClickEvent));
                Check(compact.IsChecked == true && Core.Load(path).Compact, "Failed checkbox saves must restore the saved selection.");
                fields[0].Text = "940"; fields[0].RaiseEvent(new RoutedEventArgs(UIElement.LostFocusEvent));
                Check(fields[0].Text == "940" && Core.Load(path).Width == 920, "Failed autosaves must preserve edits and disk state.");
                reject = false;
                automatic.Close();
                Check(Core.Load(path).Width == 940, "Closing Settings must save a pending field.");
            }
            finally { reject = false; automatic.Close(); }
        }
        finally { if (Directory.Exists(dir)) Directory.Delete(dir, true); }
    }
}
