// JSON mapping for the Rust share types over the web transport.
//
// The generated FRB classes carry no `fromJson`/`toJson` (FRB 2.13), so the
// HTTP transport keeps the conversion in one place. Field names match the
// Rust `serde(rename_all = "camelCase")` wire shape exactly; `i64` arrives as
// a JSON number and is widened through FRB's platform adapter.

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64, PlatformInt64Util;

import 'package:p2p_app_flutter/src/rust/api.dart'
    show CreateShareParams, MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareEntry, ShareKind, ShareManifest, ShareOffer;

MobileShare mobileShareFromJson(Map<String, dynamic> m) => MobileShare(
  shareId: m['shareId'] as String,
  ownerUserId: m['ownerUserId'] as String,
  name: m['name'] as String,
  kind: m['kind'] as String,
  publicWrite: m['publicWrite'] as bool,
  localRoot: m['localRoot'] as String?,
  revision: PlatformInt64Util.from(m['revision'] as int),
);

MobileShareGrant mobileShareGrantFromJson(Map<String, dynamic> m) =>
    MobileShareGrant(userId: m['userId'] as String, mode: m['mode'] as String);

ShareEntry shareEntryFromJson(Map<String, dynamic> m) => ShareEntry(
  path: m['path'] as String,
  fileId: m['fileId'] as String,
  size: BigInt.from(m['size'] as int),
  mime: m['mime'] as String?,
);

ShareManifest shareManifestFromJson(Map<String, dynamic> m) => ShareManifest(
  shareId: m['shareId'] as String,
  name: m['name'] as String,
  revision: BigInt.from(m['revision'] as int),
  entries: (m['entries'] as List<dynamic>)
      .map((e) => shareEntryFromJson(e as Map<String, dynamic>))
      .toList(),
);

ShareOffer shareOfferFromJson(Map<String, dynamic> m) => ShareOffer(
  shareId: m['shareId'] as String,
  name: m['name'] as String,
  kind: ShareKind.values.byName(m['kind'] as String),
  revision: BigInt.from(m['revision'] as int),
  publicWrite: m['publicWrite'] as bool,
  ownerUserId: m['ownerUserId'] as String?,
);

Map<String, dynamic> createShareParamsToJson(CreateShareParams params) => {
  'name': params.name,
  'kind': params.kind,
  'publicWrite': params.publicWrite,
  if (params.localRoot != null) 'localRoot': params.localRoot,
};

Map<String, dynamic> bindShareParamsToJson(String shareId, String? localRoot) =>
    {'shareId': shareId, 'localRoot': ?localRoot};

Map<String, dynamic> shareManifestToJson(ShareManifest manifest) => {
  'shareId': manifest.shareId,
  'name': manifest.name,
  'revision': manifest.revision.toInt(),
  'entries': manifest.entries
      .map(
        (e) => {
          'path': e.path,
          'fileId': e.fileId,
          'size': e.size.toInt(),
          'mime': ?e.mime,
        },
      )
      .toList(),
};

Map<String, dynamic> shareOfferToJson(ShareOffer offer) => {
  'shareId': offer.shareId,
  'name': offer.name,
  'kind': offer.kind.name,
  'revision': offer.revision.toInt(),
  'publicWrite': offer.publicWrite,
  if (offer.ownerUserId != null) 'ownerUserId': offer.ownerUserId,
};

/// The manifest model uses `BigInt` on every platform; FRB uses the platform's
/// native 64-bit integer. Keep that conversion at the facade boundary so widget
/// code can stay platform-independent.
PlatformInt64 shareRevisionToPlatformInt64(BigInt revision) =>
    PlatformInt64Util.from(revision.toInt());
