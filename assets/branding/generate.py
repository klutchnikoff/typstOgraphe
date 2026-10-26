#!/usr/bin/env python3
"""Build the approved tO assets and native app resources with Python 3 and rsvg-convert.

Run: python3 assets/branding/generate.py
The macOS/Windows containers use PNG layers; the ICO starts with its 32px layer.
"""

from pathlib import Path
import shutil
import struct
import subprocess
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parent
SVG = "http://www.w3.org/2000/svg"
ET.register_namespace("", SVG)
source = ET.parse(ROOT / "ligature.svg").getroot()
group = source.find(f"{{{SVG}}}g")
mark = ET.tostring(group, encoding="unicode").replace(f' xmlns="{SVG}"', "")


def optical_mark(size):
    """Use a dedicated drawing on the logical pixel grid, including Retina layers."""
    if size not in (16, 24, 32, 48):
        return mark
    tree = ET.parse(ROOT / "optical" / f"{size}.svg").getroot()
    return ET.tostring(tree.find(f"{{{SVG}}}g"), encoding="unicode").replace(f' xmlns="{SVG}"', "")


def icon(platform, size=1024):
    small = {16: (1, 14, 3), 24: (2, 20, 5), 32: (2, 28, 6), 48: (4, 40, 9)}
    dimension = size if size in small else 512
    inset, width, radius = small.get(size, (48, 416, 96 if platform == "macos" else 88))
    if platform == "macos":
        plate = '''<defs><linearGradient id="ink" x1="0" y1="0" x2=".6" y2="1">
          <stop stop-color="#39728d"/><stop offset="1" stop-color="#204c65"/>
        </linearGradient></defs>'''
        plate += f'<rect x="{inset}" y="{inset}" width="{width}" height="{width}" rx="{radius}" fill="url(#ink)"/>'
        if size not in small:
            plate += '''<rect x="49" y="49" width="414" height="414" rx="95" fill="none" stroke="#fff" stroke-opacity=".16"/>
            <path d="M144 49H368C421 49 463 91 463 144" fill="none" stroke="#fff" stroke-opacity=".1"/>'''
    else:
        # A restrained plate keeps contrast in light/dark Windows and Linux shells.
        plate = f'<rect x="{inset}" y="{inset}" width="{width}" height="{width}" rx="{radius}" fill="#285c77"/>'
    return f'''<svg xmlns="{SVG}" width="{dimension}" height="{dimension}" viewBox="0 0 {dimension} {dimension}" color="#f8f5ec">
      <title>typstOgraphe — {platform}</title>{plate}{optical_mark(size)}</svg>'''


def png(svg, size, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(target)],
        input=svg.encode(), check=True,
    )
    return target.read_bytes()


def build():
    for platform in ("macos", "windows", "linux"):
        (ROOT / f"icon-{platform}.svg").write_text(icon(platform), encoding="utf-8")
        for size in (16, 24, 32, 48, 64, 128, 256, 512, 1024):
            png(icon(platform, size), size, ROOT / "exports" / platform / f"{size}.png")
            if size < 1024:
                png(icon(platform, size), size * 2, ROOT / "exports" / platform / f"{size}@2x.png")
            if size in (16, 24, 32, 48):
                (ROOT / "exports" / platform / f"{size}.svg").write_text(icon(platform, size), encoding="utf-8")

    windows = ROOT / "exports" / "windows"
    sizes = (32, 16, 24, 48, 64, 128, 256)
    layers = [(size, (windows / f"{size}.png").read_bytes()) for size in sizes]
    offset = 6 + 16 * len(layers)
    directory, payload = bytearray(), bytearray()
    for size, data in layers:
        directory.extend(struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset))
        payload.extend(data)
        offset += len(data)
    (windows / "icon.ico").write_bytes(struct.pack("<HHH", 0, 1, len(layers)) + directory + payload)

    mac = ROOT / "exports" / "macos"
    chunks = bytearray()
    for kind, filename in (("icp4", "16"), ("icp5", "32"), ("icp6", "64"), ("ic07", "128"),
                           ("ic08", "256"), ("ic09", "512"), ("ic10", "512@2x"),
                           ("ic11", "16@2x"), ("ic12", "32@2x"), ("ic13", "128@2x"), ("ic14", "256@2x")):
        data = (mac / f"{filename}.png").read_bytes()
        chunks.extend(kind.encode() + struct.pack(">I", len(data) + 8) + data)
    (mac / "icon.icns").write_bytes(b"icns" + struct.pack(">I", len(chunks) + 8) + chunks)

    linux = ROOT / "exports" / "linux"
    (linux / "typstographe.svg").write_text(icon("linux"), encoding="utf-8")
    for size in (16, 24, 32, 48, 64, 128, 256, 512):
        target = linux / "hicolor" / f"{size}x{size}" / "apps" / "typstographe.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((linux / f"{size}.png").read_bytes())
    scalable = linux / "hicolor" / "scalable" / "apps"
    scalable.mkdir(parents=True, exist_ok=True)
    (scalable / "typstographe.svg").write_text(icon("linux"), encoding="utf-8")

    # Mask each filled letter separately: the silhouette stays identical to the icon.
    animated = ET.fromstring(mark)
    for path in animated:
        path.set("mask", f'url(#reveal-{path.attrib["id"]})')
    animated_mark = ET.tostring(animated, encoding="unicode").replace(f' xmlns="{SVG}"', "")
    template = (ROOT / "splash.template.html").read_text(encoding="utf-8")
    splash = template.replace("<!-- LIGATURE -->", animated_mark)
    (ROOT / "splash.html").write_text(splash, encoding="utf-8")
    (ROOT / "splash.svg").write_text(splash[splash.index('<svg '):splash.index('</svg>') + 6], encoding="utf-8")

    # Keep native resources in sync with the approved, size-specific exports.
    native = ROOT.parents[1] / "experiments" / "web-editor" / "src-tauri" / "icons"
    for platform in ("macos", "windows", "linux"):
        destination = native / platform
        destination.mkdir(parents=True, exist_ok=True)
        for filename, source in (("32x32.png", "32.png"), ("128x128.png", "128.png"),
                                 ("128x128@2x.png", "128@2x.png"), ("icon.png", "512.png")):
            shutil.copyfile(ROOT / "exports" / platform / source, destination / filename)
    shutil.copyfile(ROOT / "exports" / "macos" / "icon.icns", native / "icon.icns")
    shutil.copyfile(ROOT / "exports" / "windows" / "icon.ico", native / "icon.ico")
    shutil.copyfile(ROOT / "exports" / "linux" / "512.png", native / "icon.png")
    print("tO: SVG, PNG, ICO, ICNS et splash générés dans assets/branding/.")


if __name__ == "__main__":
    build()
