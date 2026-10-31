import os
import subprocess

BASE_ADDRESS = 0x80400000
STEP = 0x20000
LINKER = "src/linker.ld"

apps = os.listdir("src/bin")
apps.sort()

for app_id, app in enumerate(apps):
    app = app[: app.find(".")]
    lines: list[str] = []
    lines_before: list[str] = []
    with open(LINKER, "r") as f:
        for line in f:
            lines_before.append(line)
            line = line.replace(hex(BASE_ADDRESS), hex(BASE_ADDRESS + STEP * app_id))
            lines.append(line)
    with open(LINKER, "w+") as f:
        f.writelines(lines)
    subprocess.run(["cargo", "build", "--bin", app, "--release"], check=False)
    print(
        f"[build.py] application {app} start with address {hex(BASE_ADDRESS + STEP * app_id)}"
    )
    with open(LINKER, "w+") as f:
        f.writelines(lines_before)
