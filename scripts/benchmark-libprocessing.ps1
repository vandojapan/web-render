param([int]$Samples = 120, [string]$Proof = 'docs/proofs/libprocessing/isolated')
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskExe = Join-Path $taskRoot 'native/libprocessing/target/release/examples/smooth_probe.exe'
$taskProof = [IO.Path]::GetFullPath((Join-Path $taskRoot $Proof))
New-Item -ItemType Directory -Force $taskProof | Out-Null
$taskGpuProcess = $null
$taskRecords = @()
try {
    $taskSmi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
    if ($taskSmi) {
        $taskGpuLog = Join-Path $taskProof 'gpu.csv'
        $taskGpuProcess = Start-Process -FilePath $taskSmi.Source -ArgumentList @('--query-gpu=timestamp,index,utilization.gpu,memory.used,memory.total','--format=csv,noheader,nounits','--loop-ms=100',"--filename=$taskGpuLog") -PassThru -WindowStyle Hidden
    }
    foreach ($taskSize in @(@(1920,1080),@(3840,2160))) {
        foreach ($taskScene in @('shapes','text','image')) {
            foreach ($taskMode in @('default','no-smooth')) {
                $taskLabel = "$taskMode-$($taskSize[0])-$taskScene"
                $taskPrefix = Join-Path $taskProof $taskLabel
                $taskOut = "$taskPrefix.log"
                $taskWatch = [Diagnostics.Stopwatch]::StartNew()
                $taskStarted = [DateTimeOffset]::Now.ToString('o')
                $taskProcess = Start-Process -FilePath $taskExe -ArgumentList @($taskSize[0],$taskSize[1],$taskScene,$taskPrefix,$Samples,$taskMode) -WorkingDirectory $taskRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput $taskOut -RedirectStandardError "$taskPrefix.stderr.log"
                # Keep the Windows process handle while it is alive; otherwise
                # PowerShell 5 may return null ExitCode after an asynchronous run.
                $null = $taskProcess.Handle
                $taskPeakWorking = 0L; $taskPeakPrivate = 0L; $taskLastCpu = 0.
                $taskBenchCpu = $null; $taskBenchWall = $null; $taskBenchStart = $null
                $taskBenchEndCpu = $null; $taskBenchEndWall = $null; $taskBenchEnd = $null
                while (-not $taskProcess.WaitForExit(50)) {
                    $taskProcess.Refresh()
                    $taskPeakWorking = [Math]::Max($taskPeakWorking,$taskProcess.WorkingSet64)
                    $taskPeakPrivate = [Math]::Max($taskPeakPrivate,$taskProcess.PrivateMemorySize64)
                    $taskLastCpu = $taskProcess.TotalProcessorTime.TotalMilliseconds
                    if ($null -eq $taskBenchCpu -and (Test-Path $taskOut) -and (Select-String -Path $taskOut -Pattern '^BENCH_START$' -Quiet)) {
                        $taskBenchCpu = $taskLastCpu; $taskBenchWall = $taskWatch.Elapsed.TotalMilliseconds
                        $taskBenchStart = [DateTimeOffset]::Now.ToString('o')
                    }
                    if ($null -eq $taskBenchEndCpu -and $null -ne $taskBenchCpu -and (Select-String -Path $taskOut -Pattern '^BENCH_END$' -Quiet)) {
                        $taskBenchEndCpu = $taskLastCpu; $taskBenchEndWall = $taskWatch.Elapsed.TotalMilliseconds
                        $taskBenchEnd = [DateTimeOffset]::Now.ToString('o')
                    }
                }
                $taskProcess.WaitForExit()
                if ($taskProcess.ExitCode -ne 0) {throw "$taskLabel failed; see $taskPrefix.stderr.log"}
                if ($null -eq $taskBenchEndCpu) {$taskBenchEndCpu=$taskLastCpu; $taskBenchEndWall=$taskWatch.Elapsed.TotalMilliseconds}
                $taskActiveCpu = if ($null -ne $taskBenchCpu) {$taskBenchEndCpu-$taskBenchCpu} else {$null}
                $taskActiveWall = if ($null -ne $taskBenchWall) {$taskBenchEndWall-$taskBenchWall} else {$null}
                $taskRecords += [pscustomobject]@{label=$taskLabel;pid=$taskProcess.Id;started=$taskStarted;finished=[DateTimeOffset]::Now.ToString('o');steady_start_observed=$taskBenchStart;steady_end_observed=$taskBenchEnd;exit_code=$taskProcess.ExitCode;wall_ms=$taskWatch.Elapsed.TotalMilliseconds;cpu_ms_before_last_poll=$taskLastCpu;steady_cpu_ms_observed=$taskActiveCpu;steady_wall_ms_observed=$taskActiveWall;peak_working_set_bytes=$taskPeakWorking;peak_private_bytes=$taskPeakPrivate;sampling_ms=50}
                Write-Host "$taskLabel complete"
            }
        }
    }
} finally {
    if ($taskGpuProcess -and -not $taskGpuProcess.HasExited) {$taskGpuProcess.Kill();$taskGpuProcess.WaitForExit()}
    $taskRecords | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskProof 'process-metrics.json') -Encoding UTF8
}
