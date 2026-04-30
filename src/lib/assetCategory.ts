// Asset category classifier for pak/utoc entries. Pure path + extension rules
// derived from 0xSaturno/rivals-asset-logger's class-name buckets, mapped to
// disk-side path conventions used by Marvel Rivals.

export type AssetCategory =
  | "material"
  | "niagara"
  | "mesh"
  | "texture"
  | "animation"
  | "wwise"
  | "sound"
  | "blueprint"
  | "widget"
  | "level"
  | "data_asset"
  | "ability"
  | "movie"
  | "config"
  | "other";

export const ALL_CATEGORIES: AssetCategory[] = [
  "material",
  "niagara",
  "mesh",
  "texture",
  "animation",
  "wwise",
  "sound",
  "blueprint",
  "widget",
  "level",
  "data_asset",
  "ability",
  "movie",
  "config",
  "other",
];

export const CATEGORY_LABEL: Record<AssetCategory, string> = {
  material: "Material",
  niagara: "Niagara",
  mesh: "Mesh",
  texture: "Texture",
  animation: "Animation",
  wwise: "Wwise",
  sound: "Sound",
  blueprint: "Blueprint",
  widget: "Widget",
  level: "Level",
  data_asset: "Data",
  ability: "Ability",
  movie: "Movie",
  config: "Config",
  other: "Other",
};

// Tailwind text colors approximating Saturno's ImGui palette in AssetLoggerGUI.cpp.
export const CATEGORY_TEXT_COLOR: Record<AssetCategory, string> = {
  material: "text-yellow-400",
  niagara: "text-cyan-400",
  mesh: "text-green-400",
  texture: "text-fuchsia-400",
  animation: "text-blue-400",
  wwise: "text-pink-400",
  sound: "text-orange-400",
  blueprint: "text-red-400",
  widget: "text-teal-400",
  level: "text-rose-400",
  data_asset: "text-emerald-400",
  ability: "text-purple-400",
  movie: "text-indigo-400",
  config: "text-slate-400",
  other: "text-muted-foreground",
};

// Matching tinted-background pair for chip pills.
export const CATEGORY_BG_COLOR: Record<AssetCategory, string> = {
  material: "bg-yellow-400/15",
  niagara: "bg-cyan-400/15",
  mesh: "bg-green-400/15",
  texture: "bg-fuchsia-400/15",
  animation: "bg-blue-400/15",
  wwise: "bg-pink-400/15",
  sound: "bg-orange-400/15",
  blueprint: "bg-red-400/15",
  widget: "bg-teal-400/15",
  level: "bg-rose-400/15",
  data_asset: "bg-emerald-400/15",
  ability: "bg-purple-400/15",
  movie: "bg-indigo-400/15",
  config: "bg-slate-400/15",
  other: "bg-muted-foreground/15",
};

/**
 * Classify a pak/utoc entry path into a single category. First-match-wins; rule
 * order encodes specificity (e.g. AbilitySystem before generic Materials so a
 * /AbilitySystem/.../Materials/ path lands in Ability, not Material).
 */
export function classifyAssetPath(path: string): AssetCategory {
  const normalized = path.replace(/\\/g, "/").toLowerCase();
  const fileName = normalized.split("/").pop() ?? normalized;
  const ext = fileName.includes(".") ? (fileName.split(".").pop() ?? "") : "";

  // Extension wins for unambiguous file types.
  if (ext === "umap") return "level";
  if (ext === "wem" || ext === "bnk") return "wwise";
  if (ext === "wav" || ext === "ogg") return "sound";
  if (ext === "png" || ext === "jpg" || ext === "jpeg" || ext === "tga" || ext === "svg")
    return "texture";
  if (ext === "ttf" || ext === "ufont") return "widget";
  if (ext === "mp4" || ext === "mov" || ext === "webm" || ext === "avi" || ext === "mkv") {
    return "movie";
  }
  // Config: INI only. Aligns with the rest of the app which only edits .ini.
  if (ext === "ini") return "config";
  // Data files: plugin manifests, project, localization, ICU internationalization
  // data, CSV, dict, font third-party metadata.
  if (
    ext === "uplugin" ||
    ext === "uproject" ||
    ext === "upluginmanifest" ||
    ext === "locres" ||
    ext === "locmeta" ||
    ext === "csv" ||
    ext === "udic" ||
    ext === "upipelinecache" ||
    ext === "bin" ||
    ext === "tps" ||
    ext === "res" ||
    ext === "icu" ||
    ext === "cfu" ||
    ext === "nrm" ||
    ext === "brk" ||
    ext === "dict" ||
    ext === "ushaderbytecode"
  ) {
    return "data_asset";
  }

  // Path-segment rules. Order matters.
  if (
    normalized.includes("/wwiseaudio/") ||
    normalized.includes("/audio/banks/") ||
    normalized.includes("/akevent") ||
    normalized.includes("/plugins/wwise/") ||
    normalized.includes("/wwise/")
  ) {
    return "wwise";
  }

  if (normalized.includes("/abilitysystem/") || /\/abilit(y|ies)\//.test(normalized)) {
    return "ability";
  }

  if (
    normalized.includes("/ui/textures/") ||
    normalized.includes("/textures/") ||
    normalized.includes("/texture/") ||
    normalized.includes("/tex/") ||
    normalized.includes("/glints/") ||
    normalized.includes("/editorresources/") ||
    normalized.includes("/slate/starship/") ||
    normalized.includes("/engineresources/") ||
    normalized.includes("/enginesky/") ||
    normalized.includes("/enginelightprofiles/") ||
    normalized.includes("/editorlandscaperesources/") ||
    normalized.includes("/ies/") ||
    normalized.includes("/lightingres/") ||
    normalized.includes("/hdri/") ||
    /^(t_|ta_|tc_|rt_|tex_)/.test(fileName) ||
    fileName.includes("cubemap") ||
    /_a(tl|lt)as\d*\.uasset$/.test(fileName) ||
    /_lut\.uasset$/.test(fileName) ||
    normalized.includes("/wakanda/pv/")
  ) {
    return "texture";
  }

  // AI assets: behavior trees, blackboards, EQS, AI logic.
  // Must run before broad path/name rules so /AI/ files land in blueprint.
  if (
    /^(bt_|bb_|btt_|btd_|bts_|be_|eqs_|eqc_|eqt_)/.test(fileName) ||
    /^(aiability|aimovecontrol|aiselect|aicondition|aigameplaytaglogic|navlinkmove)/.test(
      fileName
    ) ||
    normalized.includes("/marvel/ai/") ||
    normalized.includes("/aiconfig/") ||
    normalized.includes("/aiselectarget/")
  ) {
    return "blueprint";
  }

  // UI fonts and font faces.
  if (
    normalized.includes("/marvel/font/") ||
    normalized.includes("/enginefonts/") ||
    normalized.includes("/slate/fonts/") ||
    normalized.includes("/ui/widget") ||
    normalized.includes("/umg/") ||
    normalized.includes("/mobileresources/") ||
    normalized.includes("/commonui/") ||
    normalized.includes("/uibprefcontent/") ||
    normalized.includes("/userinterface/") ||
    fileName.startsWith("wbp_") ||
    fileName.startsWith("font_") ||
    fileName.startsWith("ui_")
  ) {
    return "widget";
  }

  // Built map/level data and map templates.
  if (
    /_builtdata\.uasset$/.test(fileName) ||
    normalized.includes("/maptemplates/") ||
    normalized.includes("/maps/") ||
    normalized.includes("/map/") ||
    normalized.includes("/levels/") ||
    normalized.includes("/level/") ||
    normalized.includes("/sequence/") ||
    normalized.includes("/levelsequence/") ||
    normalized.includes("/trainlevelsequence/") ||
    normalized.includes("/subscenes/")
  ) {
    return "level";
  }

  // Mesh signals must run before /vfx/ catch-all so VFX/Meshes/SM_* doesn't
  // get swept into Niagara.
  if (
    fileName === "mesh.uasset" ||
    normalized.includes("/meshes/") ||
    normalized.includes("/mesh/") ||
    normalized.includes("/enginemeshes/") ||
    normalized.includes("/editormeshes/") ||
    normalized.includes("/basicshapes/") ||
    normalized.includes("/houdiniassets/") ||
    normalized.includes("/geometries/") ||
    normalized.includes("/datasmith/") ||
    normalized.includes("/customprops/") ||
    normalized.includes("/costommesh/") ||
    normalized.includes("/movecollisionmesh_") ||
    normalized.includes("/wakanda/temp/") ||
    /^(sm_|sk_|skm_|sdf_|cl_|dm_|supergrid_|interactor_)/.test(fileName) ||
    /_sdf\.uasset$/.test(fileName) ||
    fileName.includes("staticmesh")
  ) {
    return "mesh";
  }

  // LevelSequence
  if (fileName.startsWith("ls_")) return "level";

  // Material functions
  if (fileName.startsWith("mf_")) return "material";

  // Animation data assets, anim blueprints, RBF solvers, IK rigs, retargeters,
  // anim subgraphs, camera shakes, ground motions, skeletons. Must run before
  // generic Blueprint catch-all so AnimBP files land in animation.
  if (
    /(animdata|motiondata|rbfsolver|soulaniminfo|spinecurves|landprediction|animinfo|groundmotion|skeleton)/.test(
      fileName
    ) ||
    fileName.startsWith("ikrig_") ||
    fileName.startsWith("anim_") ||
    fileName.startsWith("a_") ||
    fileName.startsWith("ans_") ||
    fileName.startsWith("camerashake_") ||
    fileName.startsWith("cs_") ||
    /(retargeter|_retarget)\.uasset$/.test(fileName) ||
    /animbp\d*\.uasset$/.test(fileName) ||
    /_mt\.uasset$/.test(fileName) ||
    /(_anim|_montage)/.test(fileName) ||
    normalized.includes("/animations/") ||
    normalized.includes("/animation/") ||
    normalized.includes("/anim/") ||
    normalized.includes("/common/subgraphs/") ||
    normalized.includes("/animationlayerinterface/") ||
    normalized.includes("/common/rig/") ||
    normalized.includes("/camera/camerashake/") ||
    normalized.includes("/bakedanimdata/") ||
    normalized.includes("/spineanimation/") ||
    normalized.includes("/spine/")
  ) {
    return "animation";
  }

  if (
    normalized.includes("/vfx/materials/") ||
    normalized.includes("/materials/") ||
    normalized.includes("/material/") ||
    normalized.includes("/enginematerials/") ||
    normalized.includes("/editormaterials/") ||
    normalized.includes("materialfunctions") ||
    normalized.includes("materiallayerfunctions") ||
    normalized.includes("/enginedebugmaterials/") ||
    normalized.includes("/materiallibrary/") ||
    normalized.includes("/materialparamscollection/") ||
    normalized.includes("/spineplugin/") ||
    normalized.includes("/paper2d/") ||
    /(_mi_|_mat_|_mat\.|^mi_|^m_|^mm_|^ml_|^mlb_|^mfb_|^mpc_|^mc_|^mtl_|^uv_|^matlayerblend_)/.test(
      fileName
    ) ||
    /(breakoutfloat|cheapcontrast|hueshift|parallaxocclu|preserve_color)/.test(fileName) ||
    fileName.startsWith("mx_") ||
    normalized.includes("/skelot/content/")
  ) {
    return "material";
  }

  if (
    normalized.includes("/niagara/") ||
    normalized.includes("/vfx/") ||
    normalized.includes("/particles/") ||
    fileName.startsWith("pm_") ||
    fileName.startsWith("p_") ||
    /(_ns_|_fx_|^ns_|^fx_|niagara|weaponfx)/.test(fileName)
  ) {
    return "niagara";
  }

  // Curves folder: CurveFloat assets nested under Cues subdirs (must run
  // before sound-from-/cues/ so curve files don't get swept into sound).
  if (
    normalized.includes("/curves/") ||
    normalized.includes("/autoexposurecurve/") ||
    normalized.includes("/exposurecurve/") ||
    fileName.startsWith("exposure_") ||
    fileName.includes("exposure")
  ) {
    return "data_asset";
  }

  // Sound: SoundCues, voice tables, audio data, /Cues/ folders.
  if (
    fileName.startsWith("cue") ||
    /_(herovoice|voiceitem|soundtable|aeroaudio)/.test(fileName) ||
    /audiomanager\.uasset$/.test(fileName) ||
    normalized.includes("/audio/") ||
    normalized.includes("/sound/") ||
    normalized.includes("/cues/") ||
    normalized.includes("/enginesounds/")
  ) {
    return "sound";
  }

  // DataTables / data assets / curves / physics assets / settings / engine
  // dictionaries / landscape layer info / internationalization.
  if (
    fileName.startsWith("dt_") ||
    fileName.startsWith("da_") ||
    fileName.startsWith("dict_") ||
    fileName.startsWith("ca_") ||
    fileName.startsWith("cp_") ||
    fileName.startsWith("ll_") ||
    fileName.startsWith("la_") ||
    fileName.startsWith("ft_") ||
    fileName.includes("curve") ||
    /(dataasset|settings?|_layerinfo|table|struct)\.(uasset|uexp)$/.test(fileName) ||
    normalized.includes("/internationalization/") ||
    normalized.includes("/localization/") ||
    /_(abilityrestable|emoterestable|breathitem|bonelimit|copyrestable|physicsweapondata|moderuleheroitem|ruleheroitem|marvelaudiodata|sounddataasset|charactersldsettings|customtype)/.test(
      fileName
    ) ||
    /^post_.*(physics|physcis|phrsics|physice)/.test(fileName) ||
    normalized.includes("/data/") ||
    normalized.includes("/datatables/") ||
    normalized.includes("/dataasset") ||
    normalized.includes("/marvel/statistics/") ||
    normalized.includes("/interchange/runtime/content/pipelines/") ||
    normalized.includes("/foliagetype/") ||
    normalized.includes("/foliagtype/") ||
    normalized.includes("/landlayer/") ||
    normalized.includes("/layerinfo/") ||
    normalized.includes("/marvel/environment/lobby/")
  ) {
    return "data_asset";
  }

  // Weapons folder catch-all: WP_/GC_ assets and bulk data under /Weapons/
  // are dominated by mesh-related content. Must run after sound/animation
  // checks so weapon cues and montages route correctly.
  if (normalized.includes("/weapons/") || ext === "ubulk") {
    return "mesh";
  }

  // Movies: any path under /Movies(_Activity|_Level|_Skin)/. Runs after
  // material/data_asset so MI_/M_/Table/Struct uassets co-located with media
  // files route to their actual classes.
  if (/(^|\/)movies(_activity|_level|_skin)?\//.test(normalized)) {
    return "movie";
  }

  // Blueprint catch-all: matches *BP.uasset/uexp (with optional trailing digits
  // like ShowAnimBP1), substring `bp_` (CharacterBP_MassRep, ChildBP_Vanguard),
  // Entity_/Skelot_ actor blueprints, plus broad blueprint folders for
  // gameplay/editor/plugin actors that don't fit other categories.
  if (
    fileName.startsWith("bp_") ||
    fileName.startsWith("bpl_") ||
    fileName.startsWith("bs_") ||
    fileName.startsWith("fs_") ||
    fileName.startsWith("wp_") ||
    fileName.startsWith("ia_") ||
    fileName.startsWith("entity_") ||
    fileName.startsWith("skelot_") ||
    fileName.includes("bp_") ||
    /bp\d*\.uasset$/.test(fileName) ||
    /bp\d*\.uexp$/.test(fileName) ||
    /(rulecomponent|rulecontrol|processcontroller|checkpoint|guidepoint|aiguidepoint|aiguidpoint)/.test(
      fileName
    ) ||
    normalized.includes("/marvel/blueprints/") ||
    normalized.includes("/marvel/chaos/") ||
    normalized.includes("/marvel/prototype/") ||
    normalized.includes("/editorblueprintresources/") ||
    normalized.includes("/movierenderpipeline/") ||
    normalized.includes("/houdiniengine/") ||
    normalized.includes("/speedtree") ||
    normalized.includes("/vreditor/") ||
    normalized.includes("/levelgameplay/") ||
    normalized.includes("/blueprints/") ||
    normalized.includes("/blueprint/") ||
    normalized.includes("/reusable/blueprint/") ||
    normalized.includes("/gc_blueprint/") ||
    normalized.includes("/gc_blueprints/") ||
    normalized.includes("/ip_blueprint/") ||
    normalized.includes("/audio_bp/") ||
    normalized.includes("/chaosfield/") ||
    normalized.includes("/interactivefog/") ||
    normalized.includes("/fluidflux/") ||
    normalized.includes("/destruction/") ||
    normalized.includes("/vehicles/") ||
    normalized.includes("/toformal/") ||
    normalized.includes("/props/") ||
    normalized.includes("/asgard/asgarde01_s1/") ||
    normalized.includes("/gamemode/") ||
    fileName.includes("playercontroller")
  ) {
    return "blueprint";
  }

  return "other";
}
