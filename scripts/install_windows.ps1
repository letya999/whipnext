# Build and install whipnext on Windows: %LOCALAPPDATA%\whipnext + Start Menu shortcut.
# Icon is baked into the exe resources at build time; the shortcut reuses it.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "build failed" }

$appDir = Join-Path $env:LOCALAPPDATA "whipnext"
New-Item -ItemType Directory -Force -Path $appDir | Out-Null
Copy-Item "target\release\whipnext.exe" $appDir -Force
if (Test-Path "$appDir\assets") { Remove-Item "$appDir\assets" -Recurse -Force }
Copy-Item "assets" "$appDir\assets" -Recurse -Force

$programs = [Environment]::GetFolderPath("Programs")
$lnkPath = Join-Path $programs "whipnext.lnk"
$ws = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut($lnkPath)
$lnk.TargetPath = (Join-Path $appDir "whipnext.exe")
$lnk.WorkingDirectory = $appDir
$lnk.Description = "whipnext"
$lnk.Save()

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
public static class LnkAumid {
    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    public struct PropertyKey { public Guid fmtid; public uint pid;
        public PropertyKey(Guid f, uint p) { fmtid = f; pid = p; } }
    [StructLayout(LayoutKind.Explicit)]
    public struct PropVariant {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(8)] public IntPtr pwszVal; }
    [ComImport, Guid("00021401-0000-0000-C000-000000000046")]
    private class ShellLink {}
    [ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellLinkW {}
    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore {
        void GetCount(out uint c);
        void GetAt(uint i, out PropertyKey k);
        void GetValue(ref PropertyKey k, out PropVariant v);
        void SetValue(ref PropertyKey k, ref PropVariant v);
        void Commit(); }
    public static void Set(string lnk, string aumid) {
        var link = (IShellLinkW)new ShellLink();
        var pf = (IPersistFile)link;
        pf.Load(lnk, 2);
        var store = (IPropertyStore)link;
        var key = new PropertyKey(new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), 5);
        var pv = new PropVariant { vt = 31, pwszVal = Marshal.StringToCoTaskMemUni(aumid) };
        store.SetValue(ref key, ref pv);
        store.Commit();
        Marshal.FreeCoTaskMem(pv.pwszVal);
        pf.Save(lnk, true); }
}
"@
[LnkAumid]::Set($lnkPath, "whipnext.app")

Write-Host "installed: $appDir"
Write-Host "shortcut:  $lnkPath"
