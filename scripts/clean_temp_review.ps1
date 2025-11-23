# temp_output_review 폴더 삭제 (커밋 전 정리용)
Remove-Item -Path "temp_output_review" -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "✅ temp_output_review 폴더를 삭제했습니다." -ForegroundColor Green
