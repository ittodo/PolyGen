param(
    [int]$Port = 9222,
    [int]$TimeoutSeconds = 120
)

$deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)

while ([DateTime]::UtcNow -lt $deadline) {
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $client.Connect("127.0.0.1", $Port)
        Write-Host "WebView2 debugger is ready on 127.0.0.1:$Port"
        exit 0
    }
    catch {
        Start-Sleep -Milliseconds 250
    }
    finally {
        $client.Dispose()
    }
}

Write-Error "Timed out waiting for the WebView2 debugger on 127.0.0.1:$Port"
exit 1
