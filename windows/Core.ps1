# Platform-independent launcher behavior. Kept separate so it can be tested
# without opening a window or registering a system shortcut.
function ConvertTo-LumaSearchText([string]$Text) {
    return $Text.Normalize([Text.NormalizationForm]::FormC).ToLowerInvariant()
}

function Get-LumaResults($Entries, [string]$Query, $State, [bool]$Fuzzy, [int]$Limit) {
    if ($Query.Length -gt 512 -or $Query.Contains([char]0) -or $Limit -lt 1) { return @() }
    $q = ConvertTo-LumaSearchText $Query.Trim()
    $compact = $q.Replace(' ', '')
    $ranked = foreach ($entry in $Entries) {
        $name = ConvertTo-LumaSearchText $entry.Name
        $tier = 0
        if (-not $q) { $tier = 1 }
        elseif ($name -eq $q) { $tier = 7 }
        elseif ($name.StartsWith($q)) { $tier = 6 }
        elseif ($name.Contains($q)) { $tier = 5 }
        elseif ($Fuzzy) {
            $position = 0
            foreach ($c in $name.ToCharArray()) {
                if ($position -lt $compact.Length -and $c -eq $compact[$position]) { $position++ }
            }
            if ($position -eq $compact.Length) { $tier = 4 }
        }
        if ($tier -eq 0 -and $q) {
            $acronym = -join @($name -split '\s+' | Where-Object { $_ } | ForEach-Object { $_[0] })
            if ($acronym.StartsWith($q)) { $tier = 3 }
        }
        if ($tier -gt 0) {
            $usage = $State.usage[$entry.Path]
            [pscustomobject]@{ Entry = $entry; Tier = $tier; Favorite = [int]($State.favorites -contains $entry.Path); Count = if ($usage) { [long]$usage.count } else { 0 }; Last = if ($usage) { [long]$usage.last } else { 0 }; SortName = $name }
        }
    }
    return @($ranked | Sort-Object @{Expression='Tier';Descending=$true}, @{Expression='Favorite';Descending=$true}, @{Expression='Count';Descending=$true}, @{Expression='Last';Descending=$true}, SortName | Select-Object -First $Limit | ForEach-Object { $_.Entry })
}

function Get-LumaWebUrl([string]$Query) {
    $query = $Query.Trim()
    if ($query.StartsWith('g ') -and $query.Substring(2).Trim()) { return 'https://www.google.com/search?q=' + [uri]::EscapeDataString($query.Substring(2).Trim()) }
    if ($query.StartsWith('ddg ') -and $query.Substring(4).Trim()) { return 'https://duckduckgo.com/?q=' + [uri]::EscapeDataString($query.Substring(4).Trim()) }
    $uri = $null
    if ([uri]::TryCreate($query, [UriKind]::Absolute, [ref]$uri) -and $uri.Scheme -in @('http', 'https') -and $uri.Host -and -not $uri.UserInfo) { return $uri.AbsoluteUri }
    return $null
}

function Get-LumaCalculation([string]$Expression) {
    $expression = $Expression.Trim()
    if ($expression.Length -gt 256 -or $expression -notmatch '^[0-9+*/().\s-]+$') { throw 'Invalid expression' }
    $depth = 0
    foreach ($c in $expression.ToCharArray()) {
        if ($c -eq '(') { $depth++; if ($depth -gt 24) { throw 'Too many parentheses' } }
        if ($c -eq ')') { $depth--; if ($depth -lt 0) { throw 'Invalid parentheses' } }
    }
    if ($depth -ne 0) { throw 'Invalid parentheses' }
    $table = New-Object System.Data.DataTable
    $table.Locale = [Globalization.CultureInfo]::InvariantCulture
    try { $value = [double]$table.Compute($expression, '') } finally { $table.Dispose() }
    if ([double]::IsNaN($value) -or [double]::IsInfinity($value)) { throw 'Invalid result' }
    return $value.ToString('G', [Globalization.CultureInfo]::InvariantCulture)
}

function New-LumaState { return @{ favorites = @(); usage = @{}; historyEnabled = $true; fuzzyEnabled = $true } }
function Read-LumaState([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return New-LumaState }
    if ((Get-Item -LiteralPath $Path).Length -gt 4MB) { throw 'History exceeds 4 MiB' }
    $raw = Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json -ErrorAction Stop
    if ($null -eq $raw -or $null -eq $raw.favorites -or $null -eq $raw.usage) { throw 'Invalid history format' }
    $state = New-LumaState
    $state.favorites = @($raw.favorites)
    if ($null -ne $raw.historyEnabled) { $state.historyEnabled = [bool]$raw.historyEnabled }
    if ($null -ne $raw.fuzzyEnabled) { $state.fuzzyEnabled = [bool]$raw.fuzzyEnabled }
    foreach ($property in $raw.usage.PSObject.Properties) { $state.usage[$property.Name] = $property.Value }
    return $state
}
function Save-LumaState([string]$Path, $State) {
    $dir = Split-Path -Parent $Path
    [void][IO.Directory]::CreateDirectory($dir)
    $temp = Join-Path $dir ([IO.Path]::GetRandomFileName())
    $backup = Join-Path $dir ([IO.Path]::GetRandomFileName())
    try {
        $json = ConvertTo-Json -InputObject $State -Depth 6
        [IO.File]::WriteAllText($temp, $json, (New-Object Text.UTF8Encoding($false)))
        if ([IO.File]::Exists($Path)) { [IO.File]::Replace($temp, $Path, $backup) } else { [IO.File]::Move($temp, $Path) }
    } finally {
        if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Force }
        if (Test-Path -LiteralPath $backup) { Remove-Item -LiteralPath $backup -Force }
    }
}
