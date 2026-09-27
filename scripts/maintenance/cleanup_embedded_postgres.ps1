[CmdletBinding(SupportsShouldProcess)]
param(
    [int]$MinimumAgeHours = 0
)

$ErrorActionPreference = "Stop"
if ($MinimumAgeHours -lt 0) {
    throw "MinimumAgeHours must be non-negative."
}

$tempRoot = [System.IO.Path]::GetTempPath().TrimEnd('\')
$marker = 'install\18.6.0\bin\postgres.exe'
$now = Get-Date

$runningProcesses = @(Get-Process -Name postgres -ErrorAction SilentlyContinue)
$runningPaths = @()
$unknownRunningPath = $false
foreach ($process in $runningProcesses) {
    try {
        if ($process.Path) {
            $runningPaths += $process.Path
        } else {
            $unknownRunningPath = $true
        }
    } catch {
        $unknownRunningPath = $true
    }
}

$candidates = Get-ChildItem -LiteralPath $tempRoot -Force -Directory |
    Where-Object {
        $_.Name -like '.tmp*' -and
        (Test-Path -LiteralPath (Join-Path $_.FullName $marker))
    }

$removed = 0
$skippedYoung = 0
$skippedRunning = 0
$failed = 0
foreach ($candidate in $candidates) {
    if (($now - $candidate.LastWriteTime).TotalHours -lt $MinimumAgeHours) {
        $skippedYoung++
        continue
    }
    if ($unknownRunningPath -or ($runningPaths | Where-Object {
            $_.StartsWith($candidate.FullName, [System.StringComparison]::OrdinalIgnoreCase)
        })) {
        $skippedRunning++
        continue
    }
    if ($PSCmdlet.ShouldProcess($candidate.FullName, 'Remove embedded PostgreSQL test root')) {
        try {
            [System.IO.Directory]::Delete($candidate.FullName, $true)
            $removed++
        } catch {
            $failed++
            Write-Error ("Failed to remove {0}: {1}" -f $candidate.FullName, $_.Exception.Message)
        }
    }
}

Write-Output ("embedded_postgres_candidates={0} removed={1} skipped_young={2} skipped_running={3} failed={4}" -f $candidates.Count, $removed, $skippedYoung, $skippedRunning, $failed)
if ($failed -gt 0) {
    exit 1
}
