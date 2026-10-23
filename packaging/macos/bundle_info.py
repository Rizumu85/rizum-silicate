"""Write the canonical app metadata without depending on cargo-bundle defaults."""

import argparse
import plistlib
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
parser.add_argument("version")
parser.add_argument("minimum_macos")
args = parser.parse_args()

document_type = "com.rizum.silicate.procreate"
info = {
    "CFBundleIdentifier": "com.rizum.silicate",
    "CFBundleName": "Rizum Silicate",
    "CFBundleDisplayName": "Rizum Silicate",
    "CFBundleExecutable": "silicate",
    "CFBundlePackageType": "APPL",
    "CFBundleInfoDictionaryVersion": "6.0",
    "CFBundleShortVersionString": args.version,
    "CFBundleVersion": args.version,
    "CFBundleIconFile": "AppIcon",
    "LSMinimumSystemVersion": args.minimum_macos,
    "LSApplicationCategoryType": "public.app-category.graphics-design",
    "NSHighResolutionCapable": True,
    "CFBundleDocumentTypes": [{
        "CFBundleTypeName": "Procreate Artwork",
        "CFBundleTypeRole": "Viewer",
        "CFBundleTypeIconFile": "AppIcon",
        "LSHandlerRank": "Alternate",
        "LSItemContentTypes": [document_type],
    }],
    # The format is owned by Procreate, so this app imports rather than exports it.
    "UTImportedTypeDeclarations": [{
        "UTTypeIdentifier": document_type,
        "UTTypeDescription": "Procreate Artwork",
        "UTTypeConformsTo": ["public.data", "public.content"],
        "UTTypeTagSpecification": {
            "public.filename-extension": ["procreate"],
            "public.mime-type": ["application/x-procreate"],
        },
    }],
}
with args.output.open("wb") as destination:
    plistlib.dump(info, destination)
