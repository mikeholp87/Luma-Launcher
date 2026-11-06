param(
    [switch]$Toggle,
    [switch]$Background,
    [switch]$Version
)

if ($Version) { Write-Output 'Luma Launcher 0.1.0 (Windows)'; exit 0 }
if ($Toggle -and $Background) { Write-Error 'Choose either -Toggle or -Background.'; exit 2 }
if ($PSVersionTable.PSEdition -ne 'Desktop') {
    Write-Error 'Run this application with Windows PowerShell 5.1.'
    exit 1
}

Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase
. (Join-Path $PSScriptRoot 'Core.ps1')
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LumaNative {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool RegisterHotKey(IntPtr window, int id, uint modifiers, uint key);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool UnregisterHotKey(IntPtr window, int id);
}
'@

# A per-session mutex prevents two windows from competing for the hotkey.
$created = $false
$mutex = New-Object System.Threading.Mutex($true, 'Local\LumaLauncher.Windows.0.1', [ref]$created)
$signal = New-Object System.Threading.EventWaitHandle($false, [System.Threading.EventResetMode]::AutoReset, 'Local\LumaLauncher.Windows.Toggle.0.1')
if (-not $created) {
    if (-not $Background) { [void]$signal.Set() }
    $signal.Dispose()
    $mutex.Dispose()
    exit 0
}

try {
    [xml]$xaml = @'
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Title="Luma Launcher" Width="680" Height="510" WindowStartupLocation="CenterScreen" WindowStyle="None" AllowsTransparency="True" Background="Transparent" ShowInTaskbar="True">
  <Window.Resources>
    <Style TargetType="ListBoxItem">
      <Setter Property="HorizontalContentAlignment" Value="Stretch"/><Setter Property="Foreground" Value="#edf0f8"/><Setter Property="Padding" Value="12,10"/>
      <Setter Property="Template"><Setter.Value><ControlTemplate TargetType="ListBoxItem"><Border Name="Row" CornerRadius="9" Background="Transparent" BorderThickness="3,0,0,0" BorderBrush="Transparent" Padding="{TemplateBinding Padding}"><ContentPresenter/></Border><ControlTemplate.Triggers><Trigger Property="IsMouseOver" Value="True"><Setter TargetName="Row" Property="Background" Value="#282b36"/></Trigger><Trigger Property="IsSelected" Value="True"><Setter TargetName="Row" Property="Background" Value="#333c58"/><Setter TargetName="Row" Property="BorderBrush" Value="#8da9ff"/></Trigger></ControlTemplate.Triggers></ControlTemplate></Setter.Value></Setter>
    </Style>
  </Window.Resources>
  <Border Background="#20222b" BorderBrush="#4a5062" BorderThickness="1" CornerRadius="16" Padding="10">
    <Grid>
      <Grid.RowDefinitions><RowDefinition Height="Auto"/><RowDefinition Height="*"/><RowDefinition Height="Auto"/></Grid.RowDefinitions>
      <Border BorderBrush="#4a5062" BorderThickness="0,0,0,1" Padding="12,8,8,12"><Grid><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition Width="*"/><ColumnDefinition Width="Auto"/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions><TextBlock Text="&#xE721;" FontFamily="Segoe MDL2 Assets" FontSize="19" Foreground="#9ba4b8" VerticalAlignment="Center" Margin="4,0,14,0"/><Grid Grid.Column="1"><TextBox Name="Query" FontSize="19" Padding="0,8" Background="Transparent" Foreground="#edf0f8" CaretBrush="#edf0f8" BorderThickness="0" MaxLength="512" VerticalContentAlignment="Center" ToolTip="Search applications"/><TextBlock Name="Placeholder" Text="Search applications..." FontSize="19" Foreground="#9ba4b8" IsHitTestVisible="False" VerticalAlignment="Center"/></Grid><Button Name="Settings" Grid.Column="2" Content="&#xE713;" FontFamily="Segoe MDL2 Assets" ToolTip="Settings (Ctrl+,)" Background="#282b36" Foreground="#edf0f8" BorderBrush="#4a5062" Padding="10,8"/><Button Name="Quit" Grid.Column="3" Content="&#xE8BB;" FontFamily="Segoe MDL2 Assets" ToolTip="Quit Luma" Background="#282b36" Foreground="#edf0f8" BorderBrush="#4a5062" Padding="10,8" Margin="6,0,0,0"/></Grid></Border>
      <ListBox Name="Results" Grid.Row="1" Margin="0,8" FontSize="16" Background="Transparent" Foreground="#edf0f8" BorderThickness="0" ScrollViewer.HorizontalScrollBarVisibility="Disabled"><ListBox.ItemTemplate><DataTemplate><StackPanel><TextBlock Text="{Binding Display}" FontWeight="SemiBold" TextTrimming="CharacterEllipsis"/><TextBlock Text="{Binding Detail}" Foreground="#9ba4b8" FontSize="12" TextTrimming="CharacterEllipsis" Margin="0,3,0,0"/></StackPanel></DataTemplate></ListBox.ItemTemplate></ListBox>
      <Border Grid.Row="2" BorderBrush="#4a5062" BorderThickness="0,1,0,0" Padding="12,12,12,4"><TextBlock Name="Status" Foreground="#9ba4b8" FontSize="12" TextWrapping="Wrap" Text="↑↓ Navigate    Enter Open    Esc Close"/></Border>
    </Grid>
  </Border>
</Window>
'@
    $reader = New-Object System.Xml.XmlNodeReader $xaml
    $window = [Windows.Markup.XamlReader]::Load($reader)
    $query = $window.FindName('Query')
    $results = $window.FindName('Results')
    $status = $window.FindName('Status')
    $placeholder = $window.FindName('Placeholder')
    $settingsButton = $window.FindName('Settings')
    $script:entries = @()
    $script:hotkeyRegistered = $false
    $script:statePath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'LumaLauncher\history.json'
    $script:stateWritable = $true
    try { $script:state = Read-LumaState $script:statePath } catch {
        $script:state = New-LumaState
        $script:stateWritable = $false
        $status.Text = "History could not be read; it will not be overwritten: $($_.Exception.Message)"
    }
    $script:historyEnabled = $script:state.historyEnabled
    $script:fuzzyEnabled = $script:state.fuzzyEnabled

    function Update-Index {
        $roots = @(
            [Environment]::GetFolderPath('Programs'),
            [Environment]::GetFolderPath('CommonPrograms')
        ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -Unique
        $found = @(foreach ($root in $roots) {
            Get-ChildItem -LiteralPath $root -Filter '*.lnk' -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
                [pscustomobject]@{ Name = $_.BaseName; Path = $_.FullName; Kind = 'app' }
            }
        })
        $script:entries = @($found | Sort-Object Name, Path)
    }
    function Update-Results {
        $text = $query.Text.Trim()
        $placeholder.Visibility = if ($text) { 'Collapsed' } else { 'Visible' }
        $results.Items.Clear()
        if ($text.StartsWith('=')) {
            try {
                $value = Get-LumaCalculation $text.Substring(1)
                [void]$results.Items.Add([pscustomobject]@{ Kind = 'calculation'; Value = $value; Display = $value; Detail = 'Press Enter to copy result' })
            } catch { $status.Text = 'Enter a valid arithmetic expression'; return }
        } elseif ($url = Get-LumaWebUrl $text) {
            [void]$results.Items.Add([pscustomobject]@{ Kind = 'url'; Value = $url; Display = 'Open web search or URL'; Detail = $url })
        } else {
            $found = @(Get-LumaResults $script:entries $text $script:state $script:fuzzyEnabled 8)
            foreach ($item in $found) {
                $prefix = if ($script:state.favorites -contains $item.Path) { '★  ' } else { '' }
                [void]$results.Items.Add([pscustomobject]@{ Kind = 'app'; Path = $item.Path; Name = $item.Name; Display = $prefix + $item.Name; Detail = 'Application' })
            }
        }
        if ($results.Items.Count -gt 0) { $results.SelectedIndex = 0 }
        if ($script:stateWritable) { $status.Text = '↑↓ Navigate    Enter Open    Esc Close    Ctrl+D Pin' }
    }
    function Show-Palette {
        $query.Text = ''
        $window.Show()
        if ($window.WindowState -eq 'Minimized') { $window.WindowState = 'Normal' }
        [void]$window.Activate()
        [void]$query.Focus()
        $query.SelectAll()
    }
    function Toggle-Palette {
        if ($window.IsVisible) { $window.Hide() } else { Show-Palette }
    }
    function Open-Selected {
        $item = $results.SelectedItem
        if ($null -eq $item) { return }
        try {
            if ($item.Kind -eq 'calculation') {
                [Windows.Clipboard]::SetText($item.Value)
                $window.Hide()
                return
            }
            $start = New-Object System.Diagnostics.ProcessStartInfo
            $start.FileName = if ($item.Kind -eq 'url') { $item.Value } else { $item.Path }
            $start.UseShellExecute = $true
            [void][System.Diagnostics.Process]::Start($start)
            if ($item.Kind -eq 'app' -and $script:historyEnabled -and $script:stateWritable) {
                $old = $script:state.usage[$item.Path]
                $count = if ($old) { [long]$old.count + 1 } else { 1 }
                $script:state.usage[$item.Path] = @{ count = $count; last = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds() }
                Save-History
            }
            $window.Hide()
        } catch {
            $status.Text = "Could not open $($item.Name): $($_.Exception.Message)"
        }
    }
    function Save-History {
        if (-not $script:stateWritable) { $status.Text = 'History is read-only until the damaged file is repaired.'; return }
        try { Save-LumaState $script:statePath $script:state } catch { $status.Text = "Could not save history: $($_.Exception.Message)" }
    }
    function Toggle-Favorite {
        $item = $results.SelectedItem
        if (-not $item -or $item.Kind -ne 'app' -or -not $script:stateWritable) { return }
        if ($script:state.favorites -contains $item.Path) { $script:state.favorites = @($script:state.favorites | Where-Object { $_ -ne $item.Path }) }
        else { $script:state.favorites = @($script:state.favorites) + $item.Path }
        Save-History
        Update-Results
    }
    function Forget-Selected {
        $item = $results.SelectedItem
        if ($item -and $item.Kind -eq 'app' -and $script:stateWritable) { $script:state.usage.Remove($item.Path); Save-History; Update-Results }
    }
    function Show-Settings {
        $dialog = New-Object Windows.Window
        $dialog.Title = 'Luma · Settings'; $dialog.Width = 460; $dialog.Height = 340; $dialog.WindowStartupLocation = 'CenterOwner'; $dialog.Owner = $window
        $dialog.Background = [Windows.Media.BrushConverter]::new().ConvertFromString('#20222b'); $dialog.Foreground = [Windows.Media.Brushes]::White
        $panel = New-Object Windows.Controls.StackPanel; $panel.Margin = '24'
        $title = New-Object Windows.Controls.TextBlock; $title.Text = 'Settings'; $title.FontSize = 24; $title.FontWeight = 'Bold'; [void]$panel.Children.Add($title)
        $help = New-Object Windows.Controls.TextBlock; $help.Text = 'Search and history'; $help.Margin = '0,20,0,10'; $help.FontWeight = 'Bold'; [void]$panel.Children.Add($help)
        $history = New-Object Windows.Controls.CheckBox; $history.Content = 'Record launch history'; $history.IsChecked = $script:historyEnabled; $history.Foreground = [Windows.Media.Brushes]::White; $history.Margin = '0,8,0,8'; [void]$panel.Children.Add($history)
        $fuzzy = New-Object Windows.Controls.CheckBox; $fuzzy.Content = 'Fuzzy application matching'; $fuzzy.IsChecked = $script:fuzzyEnabled; $fuzzy.Foreground = [Windows.Media.Brushes]::White; $fuzzy.Margin = '0,8,0,8'; [void]$panel.Children.Add($fuzzy)
        $clear = New-Object Windows.Controls.Button; $clear.Content = 'Clear launch history'; $clear.Margin = '0,16,0,0'; $clear.Padding = '12,8'; [void]$panel.Children.Add($clear)
        $clear.Add_Click({ if ($script:stateWritable) { $script:state.usage = @{}; Save-History; $clear.Content = 'Launch history cleared' } })
        $dialog.Content = $panel
        [void]$dialog.ShowDialog()
        $script:historyEnabled = [bool]$history.IsChecked; $script:fuzzyEnabled = [bool]$fuzzy.IsChecked
        $script:state.historyEnabled = $script:historyEnabled; $script:state.fuzzyEnabled = $script:fuzzyEnabled
        Save-History
        Update-Results
        [void]$query.Focus()
    }
    $settingsButton.Add_Click({ Show-Settings })
    $window.FindName('Quit').Add_Click({ $window.Close() })
    $query.Add_TextChanged({ Update-Results })
    $results.Add_MouseDoubleClick({ Open-Selected })
    $window.Add_PreviewKeyDown({
        param($sender, $eventArgs)
        $ctrl = ([Windows.Input.Keyboard]::Modifiers -band [Windows.Input.ModifierKeys]::Control) -ne 0
        if ($ctrl) {
            switch ($eventArgs.Key.ToString()) {
                'D' { Toggle-Favorite; $eventArgs.Handled = $true; return }
                'Delete' { Forget-Selected; $eventArgs.Handled = $true; return }
                'OemComma' { Show-Settings; $eventArgs.Handled = $true; return }
                'J' { if ($results.SelectedIndex -lt $results.Items.Count - 1) { $results.SelectedIndex++ }; $eventArgs.Handled = $true; return }
                'K' { if ($results.SelectedIndex -gt 0) { $results.SelectedIndex-- }; $eventArgs.Handled = $true; return }
            }
        }
        switch ($eventArgs.Key.ToString()) {
            'Escape' { $window.Hide(); $eventArgs.Handled = $true }
            'Enter' { Open-Selected; $eventArgs.Handled = $true }
            'Down' { if ($results.SelectedIndex -lt $results.Items.Count - 1) { $results.SelectedIndex++ }; $eventArgs.Handled = $true }
            'Up' { if ($results.SelectedIndex -gt 0) { $results.SelectedIndex-- }; $eventArgs.Handled = $true }
            'F5' { Update-Index; Update-Results; $eventArgs.Handled = $true }
        }
    })
    $window.Add_SourceInitialized({
        $handle = (New-Object System.Windows.Interop.WindowInteropHelper($window)).Handle
        $source = [System.Windows.Interop.HwndSource]::FromHwnd($handle)
        $script:hook = [System.Windows.Interop.HwndSourceHook]{
            param($hwnd, $message, $wParam, $lParam, [ref]$handled)
            if ($message -eq 0x0312 -and $wParam.ToInt32() -eq 1) { Toggle-Palette; $handled.Value = $true }
            return [IntPtr]::Zero
        }
        $source.AddHook($script:hook)
        # Ctrl+Alt+Space avoids taking over the Windows Alt+Space menu.
        $script:hotkeyRegistered = [LumaNative]::RegisterHotKey($handle, 1, 0x4003, 0x20)
        if (-not $script:hotkeyRegistered) { $status.Text = 'Ctrl+Alt+Space is unavailable. Run the launcher again to toggle it.' }
    })
    $window.Add_Closed({
        $timer.Stop()
        if ($script:hotkeyRegistered) {
            $handle = (New-Object System.Windows.Interop.WindowInteropHelper($window)).Handle
            [void][LumaNative]::UnregisterHotKey($handle, 1)
        }
        [System.Windows.Threading.Dispatcher]::CurrentDispatcher.InvokeShutdown()
    })
    $timer = New-Object System.Windows.Threading.DispatcherTimer
    $timer.Interval = [TimeSpan]::FromMilliseconds(150)
    $timer.Add_Tick({ if ($signal.WaitOne(0)) { Toggle-Palette } })
    Update-Index
    Update-Results
    $timer.Start()
    if ($Background) {
        # Creating the native handle registers the shortcut without displaying the window.
        [void](New-Object System.Windows.Interop.WindowInteropHelper($window)).EnsureHandle()
    } else { Show-Palette }
    [void][System.Windows.Threading.Dispatcher]::Run()
} finally {
    $signal.Dispose()
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
