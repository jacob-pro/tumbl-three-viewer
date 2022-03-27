
docker run -it --rm -p 7007:80 ^
  -v "%~dp0nginx":/etc/nginx:ro ^
  -v "%~dp0\src":/src:ro ^
  -v "%CD%":/blogs:ro ^
  --name tumblr nginx
