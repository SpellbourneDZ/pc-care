# Build and inspect an artificial MSI database. Never install, advertise, repair, or uninstall it.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$base = Join-Path $workspace 'target/uninstall-fixtures'
foreach ($directory in @($workspace, (Join-Path $workspace 'target'), $base)) {
    if (Test-Path -LiteralPath $directory) {
        if ((Get-Item -LiteralPath $directory).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse fixture directory denied' }
    } else { $null = New-Item -ItemType Directory -Path $directory }
}
$instance = [Guid]::NewGuid().ToString()
$directory = Join-Path $base $instance
$null = New-Item -ItemType Directory -Path $directory
$productCode = '{' + [Guid]::NewGuid().ToString().ToUpperInvariant() + '}'
$packageCode = '{' + [Guid]::NewGuid().ToString().ToUpperInvariant() + '}'
$componentCode = '{' + [Guid]::NewGuid().ToString().ToUpperInvariant() + '}'
$upgradeCode = '{' + [Guid]::NewGuid().ToString().ToUpperInvariant() + '}'
$package = Join-Path $directory ('VM-ONLY-' + $instance + '.msi')
$payload = Join-Path $directory 'fixture.txt'
[IO.File]::WriteAllText($payload, "PC Care artificial uninstall fixture`r`nInstance=$instance`r`nProduct=$productCode`r`n", [Text.UTF8Encoding]::new($false))

function Call-Com($Object, [string]$Name, [Reflection.BindingFlags]$Flags, [object[]]$Arguments = @()) {
    # Automation needs native VARIANT values, not PowerShell pipeline wrappers.
    for ($index=0; $index -lt $Arguments.Count; $index++) { $Arguments[$index] = $Arguments[$index].PSObject.BaseObject }
    try { $Object.GetType().InvokeMember($Name, $Flags, $null, $Object, $Arguments) } catch { throw "MSI COM $Name ($(@($Arguments | ForEach-Object { $_.GetType().Name }) -join ',')) failed: $($_.Exception.Message)" }
}
function Release-Com($Object) {
    if ($null -ne $Object) { $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($Object) }
}
function File-Hash([string]$Path) {
    $hash = [Security.Cryptography.SHA256]::Create()
    $stream = $null
    try { $stream = [IO.File]::OpenRead($Path); ([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
    finally { if ($null -ne $stream) { $stream.Dispose() }; $hash.Dispose() }
}
function Execute-Sql([string]$Sql) {
    try { $view = Call-Com $database 'OpenView' InvokeMethod @($Sql) } catch { throw "Fixture SQL rejected: $Sql" }
    try { $null = Call-Com $view 'Execute' InvokeMethod; $null = Call-Com $view 'Close' InvokeMethod } finally { Release-Com $view }
}
function Query-Strings([string]$Sql) {
    $view = Call-Com $database 'OpenView' InvokeMethod @($Sql)
    try {
        $null = Call-Com $view 'Execute' InvokeMethod
        while ($true) {
            $record = Call-Com $view 'Fetch' InvokeMethod
            if ($null -eq $record) { break }
            try { Call-Com $record 'StringData' GetProperty @(1) } finally { Release-Com $record }
        }
        $null = Call-Com $view 'Close' InvokeMethod
    } finally { Release-Com $view }
}
$installer = $null; $database = $null; $summary = $null
try {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    if ((Call-Com $installer 'ProductState' GetProperty @($productCode)) -ne -1) { throw 'Fixture ProductCode already registered' }
    $database = Call-Com $installer 'OpenDatabase' InvokeMethod @($package, 3)
    $tables = @(
        'CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` LONGCHAR NOT NULL LOCALIZABLE PRIMARY KEY `Property`)',
        'CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72), `DefaultDir` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY `Directory`)',
        'CREATE TABLE `Component` (`Component` CHAR(72) NOT NULL, `ComponentId` CHAR(38), `Directory_` CHAR(72) NOT NULL, `Attributes` SHORT NOT NULL, `Condition` CHAR(255), `KeyPath` CHAR(72) PRIMARY KEY `Component`)',
        'CREATE TABLE `Feature` (`Feature` CHAR(38) NOT NULL, `Feature_Parent` CHAR(38), `Title` CHAR(64) LOCALIZABLE, `Description` CHAR(255) LOCALIZABLE, `Display` SHORT, `Level` SHORT NOT NULL, `Directory_` CHAR(72), `Attributes` SHORT NOT NULL PRIMARY KEY `Feature`)',
        'CREATE TABLE `FeatureComponents` (`Feature_` CHAR(38) NOT NULL, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Feature_`, `Component_`)',
        'CREATE TABLE `File` (`File` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL, `FileName` CHAR(255) NOT NULL LOCALIZABLE, `FileSize` LONG NOT NULL, `Version` CHAR(72), `Language` CHAR(20), `Attributes` SHORT, `Sequence` SHORT NOT NULL PRIMARY KEY `File`)',
        'CREATE TABLE `Media` (`DiskId` SHORT NOT NULL, `LastSequence` SHORT NOT NULL, `DiskPrompt` CHAR(64) LOCALIZABLE, `Cabinet` CHAR(255), `VolumeLabel` CHAR(32), `Source` CHAR(72) PRIMARY KEY `DiskId`)',
        'CREATE TABLE `LaunchCondition` (`Condition` CHAR(255) NOT NULL, `Description` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY `Condition`)',
        'CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT PRIMARY KEY `Action`)',
        'CREATE TABLE `InstallUISequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT PRIMARY KEY `Action`)'
    )
    foreach ($sql in $tables) { Execute-Sql $sql }
    $properties = [ordered]@{ ProductCode=$productCode; ProductName='PC Care VM Fixture'; ProductVersion='1.0.0'; Manufacturer='PC Care Fixture Lab'; ProductLanguage='1033'; UpgradeCode=$upgradeCode; ARPNOMODIFY='1'; ARPNOREPAIR='1' }
    foreach ($name in $properties.Keys) { Execute-Sql ('INSERT INTO `Property` (`Property`, `Value`) VALUES (''' + $name + ''', ''' + $properties[$name] + ''')') }
    Execute-Sql 'INSERT INTO `Directory` (`Directory`, `DefaultDir`) VALUES (''TARGETDIR'', ''SourceDir'')'
    Execute-Sql 'INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES (''LocalAppDataFolder'', ''TARGETDIR'', ''.'')'
    Execute-Sql 'INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES (''FixtureVendor'', ''LocalAppDataFolder'', ''PCCAREVM|PC Care VM Fixtures:.'')'
    Execute-Sql ('INSERT INTO `Directory` (`Directory`, `Directory_Parent`, `DefaultDir`) VALUES (''INSTALLDIR'', ''FixtureVendor'', ''FIXTURE|' + $instance + ':.'')')
    Execute-Sql ('INSERT INTO `Component` (`Component`, `ComponentId`, `Directory_`, `Attributes`, `KeyPath`) VALUES (''FixtureComponent'', ''' + $componentCode + ''', ''INSTALLDIR'', 0, ''FixtureFile'')')
    Execute-Sql 'INSERT INTO `Feature` (`Feature`, `Title`, `Display`, `Level`, `Directory_`, `Attributes`) VALUES (''Main'', ''Artificial fixture only'', 1, 1, ''INSTALLDIR'', 0)'
    Execute-Sql 'INSERT INTO `FeatureComponents` (`Feature_`, `Component_`) VALUES (''Main'', ''FixtureComponent'')'
    Execute-Sql ('INSERT INTO `File` (`File`, `Component_`, `FileName`, `FileSize`, `Attributes`, `Sequence`) VALUES (''FixtureFile'', ''FixtureComponent'', ''fixture.txt'', ' + (Get-Item -LiteralPath $payload).Length + ', 8192, 1)')
    Execute-Sql 'INSERT INTO `Media` (`DiskId`, `LastSequence`) VALUES (1, 1)'
    $condition = 'Installed OR PC_CARE_VM_FIXTURE = "' + $instance + '"'
    Execute-Sql ('INSERT INTO `LaunchCondition` (`Condition`, `Description`) VALUES (''' + $condition + ''', ''VM fixture only. Installation requires the isolated-VM test harness.'')')
    $execute = [ordered]@{ LaunchConditions=100; CostInitialize=800; FileCost=900; CostFinalize=1000; InstallValidate=1400; InstallInitialize=1500; ProcessComponents=1600; UnpublishFeatures=1800; RemoveFiles=3500; RemoveFolders=3600; InstallFiles=4000; RegisterUser=6000; RegisterProduct=6100; PublishFeatures=6300; PublishProduct=6400; InstallFinalize=6600 }
    foreach ($action in $execute.Keys) { Execute-Sql ('INSERT INTO `InstallExecuteSequence` (`Action`, `Sequence`) VALUES (''' + $action + ''', ' + $execute[$action] + ')') }
    foreach ($action in @('LaunchConditions','CostInitialize','FileCost','CostFinalize')) { Execute-Sql ('INSERT INTO `InstallUISequence` (`Action`, `Sequence`) VALUES (''' + $action + ''', ' + $execute[$action] + ')') }
    Execute-Sql 'INSERT INTO `InstallUISequence` (`Action`, `Sequence`) VALUES (''ExecuteAction'', 1300)'
    $summary = Call-Com $database 'SummaryInformation' GetProperty @(20)
    foreach ($item in @(@(1,1252),@(2,'PC Care VM fixture only'),@(3,'Artificial per-user uninstall test'),@(4,'PC Care Fixture Lab'),@(7,'Intel;1033'),@(9,$packageCode),@(14,400),@(15,8),@(18,'PC Care fixture builder'),@(19,2))) {
        $null = Call-Com $summary 'Property' SetProperty $item
    }
    $null = Call-Com $summary 'Persist' InvokeMethod
    Release-Com $summary; $summary=$null
    $null = Call-Com $database 'Commit' InvokeMethod
    Release-Com $database; $database=$null
    $database = Call-Com $installer 'OpenDatabase' InvokeMethod @($package, 0)
    $summary = Call-Com $database 'SummaryInformation' GetProperty @(0)
    if ((Call-Com $summary 'Property' GetProperty @(15)) -ne 8) { throw 'Unexpected elevated/compressed package flags' }
    Release-Com $summary; $summary=$null
    $actual = @(Query-Strings 'SELECT `Name` FROM `_Tables`')
    $expected = @('Property','Directory','Component','Feature','FeatureComponents','File','Media','LaunchCondition','InstallExecuteSequence','InstallUISequence')
    if (@(Compare-Object ($actual | Sort-Object) ($expected | Sort-Object)).Count -ne 0) { throw 'Unexpected MSI table (no custom actions/registry/services allowed)' }
    if (@(Query-Strings 'SELECT `File` FROM `File`').Count -ne 1) { throw 'Unexpected payload file count' }
    if ((Query-Strings 'SELECT `Condition` FROM `LaunchCondition`') -ne $condition) { throw 'Missing install guard' }
    if ((Query-Strings 'SELECT `Value` FROM `Property` WHERE `Property` = ''ProductCode''') -ne $productCode) { throw 'Product identity mismatch' }
    if (@(Query-Strings 'SELECT `Value` FROM `Property` WHERE `Property` = ''ALLUSERS''').Count -ne 0) { throw 'Machine-wide fixture denied' }
    if ((Call-Com $installer 'ProductState' GetProperty @($productCode)) -ne -1) { throw 'Fixture unexpectedly installed' }
    Release-Com $database; $database=$null
    $manifest = [ordered]@{
        schema_version=1
        instance=$instance
        product_code=$productCode
        package_code=$packageCode
        component_code=$componentCode
        upgrade_code=$upgradeCode
        name='PC Care VM Fixture'
        publisher='PC Care Fixture Lab'
        version='1.0.0'
        scope='current_user'
        vm_only=$true
        installed_on_host=$false
        package=(Split-Path $package -Leaf)
        payload='fixture.txt'
        destination_suffix=(Join-Path 'PC Care VM Fixtures' $instance)
        required_install_property=('PC_CARE_VM_FIXTURE=' + $instance)
        msi_sha256=(File-Hash $package)
        payload_sha256=(File-Hash $payload)
        tables=$expected
    }
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $directory 'manifest.json') -Encoding UTF8
    Write-Output ($manifest | ConvertTo-Json -Depth 4 -Compress)
} finally { Release-Com $summary; Release-Com $database; Release-Com $installer }
