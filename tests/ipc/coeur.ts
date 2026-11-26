// Généré par `ARNOD_IPC_ECRIRE=1 cargo test --lib ts_mirror` (src-tauri/src/ts_mirror.rs) :
// forme TypeScript des types sérialisés du cœur. Ne pas modifier à la main.

import type * as Ipc from "@/types/ipc";

export type ActiveRequest = { projectId: string, path: string, };

export type AddedHeader = { name: string, value: string, origin: AddedOrigin, source: string | null, };

export type AddedOrigin = "auth" | "body" | "client";

export type ApiKeyLocation = "header" | "query";

export type Assertion = { id: string, target: string, operator: AssertionOperator, value?: Scalar | null, origin: AssertionOrigin, enabled: boolean, };

export type AssertionOperator = "equals" | "notEquals" | "exists" | "contains" | "notContains" | "lessThan" | "lessOrEqual" | "greaterThan" | "greaterOrEqual" | "in" | "notIn" | "between" | "startsWith" | "endsWith" | "length" | "matches" | "notMatches" | "isNull" | "isUndefined" | "isEmpty" | "isNotEmpty" | "isTruthy" | "isFalsy" | "isJson" | "isNumber" | "isText" | "isBoolean" | "isArray";

export type AssertionOrigin = "proposed" | "manual" | "default";

export type AssertionResult = { id: string, target: string, passed: boolean, message: string | null, actual: string | null, ignored: boolean, };

export type Auth = { type: AuthKind, username: string, password: string, token: string, name: string, value: string, location: ApiKeyLocation, other: string, oauth2: OAuth2, };

export type AuthKind = "none" | "basic" | "bearer" | "apiKey" | "inherit" | "other" | "oauth2";

export type AuthOverride = { label: string, path: string, type: AuthKind, other: string, count: number, };

export type AuthUsage = { inheriting: number, overrides: Array<AuthOverride>, };

export type AuthorityView = { addedAt: number, subject: string, issuer: string, notBefore: string, notAfter: string, fingerprint: string, names: Array<string>, authority: boolean, selfSigned: boolean, pem: string, };

export type Axis = "row" | "column";

export type BaseUrlSet = { environment: string, value: string, };

export type BatchAction = { "kind": "move", target: string, } | { "kind": "duplicate" } | { "kind": "delete", permanent: boolean, };

export type BatchDone = { items: Array<[string, string | null]>, };

export type BatchResult = { items: Array<[string, string | null]>, undoToken: string, };

export type Body = { kind: BodyKind, variants: Array<BodyVariant>, selected: number, other: string, };

export type BodyKind = "none" | "json" | "text" | "xml" | "form" | "other";

export type BodyToken = { text: string, kind: TokenKind, path?: string, };

export type BodyVariant = { title: string, content: string, form: Array<Pair>, };

export type CatalogFile = { version: number, projects: Array<Project>, workspaces: Array<Workspace>, lastWorkspaceId: string | null, inboxProjectId?: string, };

export type CatalogFileV1 = { projects: Array<Project>, workspaces: Array<Workspace>, defaultWorkspaceId: string, lastWorkspaceId: string, };

export type CatalogLoad = { snapshot: CatalogSnapshot, warning: CommandError | null, };

export type CatalogSnapshot = { projects: Array<ProjectView>, workspaces: Array<WorkspaceView>, lastWorkspaceId: string | null, inboxProjectId: string | null, };

export type CertificateFailure = { problem: CertificateProblem, server: CertificateInfo, authority: CertificateInfo | null, };

export type CertificateInfo = { subject: string, issuer: string, notBefore: string, notAfter: string, fingerprint: string, names: Array<string>, authority: boolean, selfSigned: boolean, pem: string, };

export type CertificateProblem = "unknownIssuer" | "expired" | "notYetValid" | "wrongName" | "revoked" | "other";

export type ClearSecret = { field: string, label: string, suggested: string, };

export type CommandError = { kind: CommandErrorKind, message: string, failure?: NetworkFailure, };

export type CommandErrorKind = "invalidName" | "nameTaken" | "folderNotEmpty" | "alreadyRegistered" | "notAProject" | "notFound" | "undoExpired" | "io" | "corruptState" | "newerVersion" | "unknownVariable" | "invalidRequest" | "network" | "timeout" | "cancelled" | "secretStore" | "protectedEnvironment" | "changedOnDisk";

export type ConflictItem = { key: string, label: string, disk: string | null, draft: string | null, identical: boolean, suggested: ConflictSide | null, };

export type ConflictSide = "disk" | "draft";

export type ContentRef = { "kind": "request", projectId: string, path: string, } | { "kind": "run", projectId: string, } | { "kind": "contract", projectId: string, path: string, } | { "kind": "result" } | { "kind": "importReport", projectId: string, } | { "kind": "test", projectId: string, path: string, };

export type ContentTab = { id: string, content: ContentRef, preview: boolean, };

export type ContractLink = { file: string, operation: string, };

export type ContractSource = { "kind": "text", text: string, } | { "kind": "url", url: string, } | { "kind": "template", name: string, };

export type ContractSummary = { deduced: boolean, path: string, name: string, version: string, operationCount: number, error: string | null, resources: Array<ResourceView>, unplaced: Array<LinkedView>, };

export type ContractView = { deduced: boolean, path: string, name: string, version: string, kind: SpecKind | null, specVersion: string, description: string | null, servers: Array<string>, source: string, error: string | null, operations: Array<OperationView>, orphans: Array<OrphanView>, };

export type ConversionExample = { before: string, after: string, };

export type CookieDomain = { domain: string, sending: boolean, cookies: Array<CookieView>, };

export type CookieJarView = { environment: string, host: string | null, domains: Array<CookieDomain>, };

export type CookieView = { name: string, value: string, path: string, expiresAt: number | null, httpOnly: boolean, secure: boolean, receivedFrom: string, };

export type CreatedTest = { folder: string, step: string, };

export type CredentialsPlacement = "basicAuthHeader" | "body";

export type CurlCopy = { command: string, environment: string, };

export type CurlIgnored = { option: string, reason: string, };

export type CurlImport = { secrets: Array<CurlSecret>, ignored: Array<CurlIgnored>, };

export type CurlSecret = { name: string, value: string, header: string, };

export type Definition = { format: number, id: string, version: string, name: string, title: string, noun: string, description: string, family: StepFamily, publisher: string, engine: StepEngine, action: string, subject: StepSubject, detect: Array<StepRule>, form: Array<StepField>, };

export type DetailRow = { label: string, value: string, };

export type Detection = { nature: string, subject: string, values: { [key in string]: string }, curl?: CurlImport, };

export type DrawnValue = { name: string, value: string, source: string | null, };

export type DynamicValue = { name: string, description: string, example: string, };

export type EnvVariable = { name: string, value: string, };

export type EnvironmentDoc = { name: string, color: string, protected: boolean, variables: Array<EnvVariable>, locals: Array<string>, };

export type EnvironmentView = { localVars: Array<LocalVarView>, name: string, color: string, protected: boolean, variables: Array<EnvVariable>, locals: Array<string>, };

export type EnvironmentsView = { environments: Array<EnvironmentView>, active: string, };

export type Example = { name: string, description: string, status: number | null, statusText: string, headers: Array<ResponseHeader>, bodyKind: ExampleBodyKind, body: string, };

export type ExampleBodyKind = "none" | "json" | "text" | "xml" | "html" | "binary";

export type ExampleInput = { status: number, statusText: string, headers: Array<[string, string]>, body: string, };

export type ExampleRef = { index: number, status: number | null, name: string, };

export type ExtractedValue = { name: string, source: string, value: string | null, message: string | null, };

export type Extraction = { name: string, source: string, enabled: boolean, };

export type FailureCause = "certificate" | "proxy" | "proxyAuth" | "dns" | "unreachable" | "timeout" | "tls" | "other";

export type Filed = { moved: Array<[string, string]>, left: Array<string>, contract: string, contractName: string, };

export type FiledTrials = { undoToken: string, moved: Array<[string, string]>, left: Array<string>, contract: string, contractName: string, };

export type FilesChange = { projectId: string, added: Array<string>, modified: Array<string>, removed: Array<string>, renamed: Array<RenamedFile>, tree: boolean, environments: boolean, contracts: boolean, };

export type FilterMatch = { path: string, value: string, assertion: Assertion | null, };

export type FilterResult = { matches: Array<FilterMatch>, total: number, error: string | null, };

export type FolderTarget = { path: string, current: boolean, };

export type GenerateReport = { folder: string, created: Array<string>, skipped: Array<SkippedOperation>, baseUrl: BaseUrlSet | null, };

export type GhAsset = { name: string, browser_download_url: string, };

export type GhRelease = { tag_name: string, name: string | null, body: string | null, html_url: string, draft: boolean, prerelease: boolean, published_at: string | null, assets: Array<GhAsset>, };

export type HelloResponse = { message: string, timestampMs: number, };

export type HtmlNode = { "kind": "text", text: string, } | { "kind": "element", tag: string, children: Array<HtmlNode>, };

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export type HttpResponse = { status: number, statusText: string, headers: Array<[string, string]>, body: string, truncated: boolean, sizeBytes: number, waitMs: number, receiveMs: number, totalMs: number, timeline: Timeline, };

export type ImportCounts = { folders: number, requests: number, environments: number, assertions: number, extractions: number, secrets: number, dynamicValues: number, scripts: number, };

export type ImportDone = { projectId: string, snapshot: CatalogSnapshot | null, report: ImportReport | null, contract: string | null, };

export type ImportFormat = "bruno" | "openCollection" | "openApi";

export type ImportItemKind = "tests" | "preRequest" | "postResponse" | "env" | "auth" | "body" | "request" | "assertion" | "extraction" | "option";

export type ImportItemStatus = "notRun" | "toFill" | "toCheck" | "skipped" | "stored";

export type ImportReport = { collection: string, format: ImportFormat, source: string, sourceFiles: number, projectId: string, projectName: string, folder: string | null, importedAt: number, counts: ImportCounts, items: Array<ImportReportItem>, examples: Array<ConversionExample>, };

export type ImportReportItem = { kind: ImportItemKind, status: ImportItemStatus, name: string, place: string, file: string, block: string | null, request: string | null, detail: string, code: string | null, };

export type ImportSource = { path: string, format: ImportFormat, name: string, counts: ImportCounts, files: number, };

export type ImportTarget = { "kind": "newProject", location: string | null, workspaceId: string | null, } | { "kind": "project", projectId: string, };

export type Inherited = { auth: InheritedAuth | null, headers: Array<InheritedHeader>, settings: ResolvedSettings, };

export type InheritedAuth = { auth: Auth, source: string, };

export type InheritedHeader = { name: string, value: string, source: string, };

export type InstallState = { "state": "idle" } | { "state": "downloading", received: number, total: number | null, } | { "state": "ready", version: string, onExit: boolean, } | { "state": "failed", message: string, };

export type Jar = { cookies: Array<StoredCookie>, disabledDomains: Array<string>, };

export type LayoutNode = { "kind": "zone" } & ZoneView | { "kind": "split", id: string, axis: Axis, ratio: number, first: LayoutNode, second: LayoutNode, };

export type LayoutPreset = "one" | "twoColumns" | "twoRows" | "three" | "four";

export type LinkedView = { path: string, name: string, method: HttpMethod, examples: Array<ExampleRef>, };

export type LocalData = { lastEnvironment: string | null, collapsed: Array<string>, locals: { [key in string]: Array<LocalVar> }, };

export type LocalVar = { name: string, value: string | null, secret: boolean, };

export type LocalVarView = { name: string, secret: boolean, value: string | null, hasValue: boolean, };

export type ManualProxy = { url: string, noProxy: string, username: string, };

export type MenuShortcut = "placement" | "sidebar";

export type NetworkFailure = { cause: FailureCause, title: string, explanation: string, notSent: boolean, host: string, port: number, proxy: string | null, connectTimeoutMs: number, elapsedMs: number, certificate: CertificateFailure | null, };

export type NetworkPrefs = { format: number, proxy: ProxyPrefs, connectTimeoutMs: number, authorities: Array<StoredAuthority>, };

export type NetworkPrefsInput = { proxy: ProxyPrefs, proxyPassword: string | null, connectTimeoutMs: number, };

export type NetworkView = { proxy: ProxyPrefs, hasProxyPassword: boolean, system: SystemProxy, connectTimeoutMs: number, systemAuthorities: number, authorities: Array<AuthorityView>, };

export type OAuth2 = { accessTokenUrl: string, clientId: string, clientSecret: string, scope: string, credentials: CredentialsPlacement, tokenPlacement: TokenPlacement, tokenName: string, };

export type OperationNode = { key: string, method: string, path: string, summary: string | null, supported: boolean, requests: Array<LinkedView>, };

export type OperationTarget = { contract: string, contractName: string, operation: string, current: boolean, };

export type OperationView = { key: string, method: string, path: string, summary: string | null, operationId: string | null, tag: string | null, deprecated: boolean, supported: boolean, externalRefs: boolean, request: string | null, };

export type OrphanView = { path: string, name: string, operation: string, };

export type Pair = { name: string, value: string, enabled: boolean, kind: ParamKind, description: string, };

export type PairsFormat = "header" | "query";

export type PairsText = { pairs: Array<Pair>, errors: Array<TextError>, };

export type ParamKind = "query" | "path";

export type PathGeneralization = { path: string, template: Template, contract: string, };

export type PathParam = { name: string, value: string, };

export type Placement = "auto" | "right" | "below";

export type ProblemNature = "newerVersion" | "corruptState" | "locked" | "inaccessible";

export type Project = { id: string, name: string, path: string, };

export type ProjectAuthView = { auth: Auth, environment: string, token: TokenStatus | null, hasClientSecret: boolean, usage: AuthUsage, };

export type ProjectChange = { projectId: string, scope: ProjectScope, path: string | null, origin: string, renamed: Array<RenamedFile>, removed: Array<string>, };

export type ProjectCreated = { snapshot: CatalogSnapshot, projectId: string, };

export type ProjectLastRun = { projectId: string, startedAt: number, passed: number, total: number, };

export type ProjectScope = "tree" | "request" | "environments" | "contracts";

export type ProjectStatus = "ok" | "missing";

export type ProjectView = { id: string, name: string, path: string, status: ProjectStatus, workspaceIds: Array<string>, };

export type ProposeInput = { status: number, durationMs: number, headers: Array<[string, string]>, body: string, };

export type ProxyMode = "system" | "none" | "manual";

export type ProxyPrefs = { mode: ProxyMode, manual: ManualProxy, };

export type ProxySource = "environment" | "windows";

export type Release = { version: string, prerelease: boolean, highlights: Array<string>, url: string, published: string | null, };

export type RenamedFile = { from: string, to: string, };

export type RenderedBody = { format: ResponseFormat, detected: ResponseFormat, lines: Array<Array<BodyToken>>, notice: string | null, };

export type RequestConflict = { items: Array<ConflictItem>, modifiedAt: number | null, };

export type RequestDoc = { name: string, description: string, tags: Array<string>, method: HttpMethod, url: string, params: Array<Pair>, headers: Array<Pair>, auth: Auth, body: Body, settings: RequestSettings, docs: string, assertions: Array<Assertion>, extractions: Array<Extraction>, seq?: number, examples: Array<Example>, contract?: ContractLink, preserved: Array<string>, inherited: Inherited, };

export type RequestInput = { environment: string, known: Array<string>, unknown: Array<UnknownVariable>, added: Array<AddedHeader>, secrets: Array<ClearSecret>, };

export type RequestResult = { projectId: string, path: string, sending: boolean, outcome: SendOutcome | null, error: string | null, failure?: NetworkFailure, receivedAt: number | null, request: RequestDoc | null, };

export type RequestSettings = { timeoutMs: number | null, followRedirects: boolean | null, maxRedirects: number | null, encodeUrl: boolean | null, };

export type RequestStatus = { passed: boolean, at: number, };

export type ResolvedSettings = { timeoutMs: number, followRedirects: boolean, maxRedirects: number, encodeUrl: boolean, };

export type ResourceView = { path: string, operations: Array<OperationNode>, };

export type ResponseFormat = "json" | "xml" | "html" | "text";

export type ResponseHeader = { name: string, value: string, };

export type ResultChange = { result: RequestResult, origin: string, };

export type RetryPlan = { attempt: number, maxAttempts: number, delaySeconds: number, };

export type RunChange = { projectId: string, run: RunView, origin: string, };

export type RunGroup = { name: string, path: string, passed: number, failed: number, total: number, };

export type RunItem = { path: string, name: string, method: HttpMethod, status: RunItemStatus, httpStatus: number | null, durationMs: number | null, slow: boolean, assertions: Array<AssertionResult>, error: string | null, test: string | null, step: number | null, extracted: Array<ExtractedValue>, };

export type RunItemStatus = "pending" | "running" | "passed" | "failed" | "error" | "cancelled" | "skipped";

export type RunTest = { path: string, name: string, status: RunItemStatus, steps: number, passed: number, failedStep: number | null, };

export type RunView = { id: string, number: number, folder: string, environment: string, concurrency: number, startedAt: number, finishedAt: number | null, finished: boolean, cancelled: boolean, items: Array<RunItem>, contract: string | null, revision: number, groups: Array<RunGroup>, tests: Array<RunTest>, };

export type Scalar = boolean | number | number | string;

export type SendOutcome = { response: HttpResponse, assertions: Array<AssertionResult>, passed: boolean, slow: boolean, sent: Sent | null, };

export type Sent = { method: string, url: string, headers: Array<[string, string]>, body: string | null, drawn: Array<DrawnValue>, used: Array<UsedVariable>, extracted: Array<ExtractedValue>, };

export type Side = "right" | "bottom";

export type SkippedOperation = { operation: string, reason: string, };

export type SpecKind = "openApi3" | "swagger2";

export type StartupAction = "openReleases" | "startEmpty" | "retry" | "openFolder";

export type StartupOption = { action: StartupAction, title: string, description: string, button: string, recommended: boolean, note: string | null, };

export type StartupProblem = { nature: ProblemNature, file: string, title: string, message: string, intactLead: string, intactText: string, options: Array<StartupOption>, pathLabel: string, path: string, retry: RetryPlan | null, details: Array<DetailRow>, supportText: string, issueUrl: string, footer: string, };

export type StartupTarget = "folder" | "releases" | "issue";

export type StatusesChange = { projectId: string, statuses: { [key in string]: RequestStatus }, };

export type StepCatalog = { source: StepCatalogSource, natures: Array<StepNature>, };

export type StepCatalogSource = { name: string, updated: string, synced: boolean, };

export type StepEngine = { "kind": "builtin", id: string, abi: number, } | { "kind": "wasm", module: string, sha256: string, abi: number, };

export type StepFamily = "calls" | "data" | "messages" | "jobs" | "interface" | "code" | "files";

export type StepField = { key: string, label: string, kind: StepFieldKind, default: string | null, placeholder: string | null, hint: string | null, options: Array<string>, visibleWhen: StepVisibleWhen | null, };

export type StepFieldKind = "choice" | "text" | "multiline";

export type StepNature = { origin: StepOrigin, format: number, id: string, version: string, name: string, title: string, noun: string, description: string, family: StepFamily, publisher: string, engine: StepEngine, action: string, subject: StepSubject, detect: Array<StepRule>, form: Array<StepField>, };

export type StepOrigin = "builtin";

export type StepRule = { "kind": "url", schemes: Array<string>, methodField: string | null, } | { "kind": "keywords", words: Array<string>, } | { "kind": "prefix", text: string, } | { "kind": "suffix", text: string, } | { "kind": "curl" };

export type StepSubject = { key: string, label: string, placeholder: string, };

export type StepView = { path: string, name: string, method: HttpMethod | null, url: string, assertionCount: number, extractions: Array<string>, invalid: boolean, };

export type StepVisibleWhen = { key: string, in: Array<string>, };

export type StoredAuthority = { pem: string, addedAt: number, };

export type StoredCookie = { name: string, value: string, domain: string, hostOnly: boolean, path: string, expiresAt: number | null, secure: boolean, httpOnly: boolean, receivedFrom: string, receivedAt: number, };

export type SystemProxy = { http: string | null, https: string | null, noProxy: string, source: ProxySource | null, autoConfig: string | null, };

export type Template = { path: string, params: Array<PathParam>, };

export type TestTarget = { path: string, name: string, steps: Array<string>, current: number | null, };

export type TestView = { path: string, name: string, test: boolean, steps: Array<StepView>, };

export type TextError = { line: number, message: string, };

export type Timeline = { dnsMs: number | null, connectMs: number | null, secure: boolean, serverMs: number, receiveMs: number, totalMs: number, };

export type TokenKind = "key" | "string" | "number" | "literal" | "punct" | "tag" | "attr" | "attrValue" | "comment" | "text";

export type TokenPlacement = "header" | "query";

export type TokenStatus = { obtainedAt: number, expiresAt: number | null, };

export type TransferChange = { title: string, detail: string, warning: boolean, };

export type TransferDone = { projectId: string, path: string, name: string, undoToken: string, };

export type TransferMode = "copy" | "move";

export type TransferTarget = { "kind": "folder", folder: string, } | { "kind": "step", test: string, position: number, } | { "kind": "operation", contract: string, operation: string, };

export type TransferTargets = { method: string, operations: Array<OperationTarget>, folders: Array<FolderTarget>, tests: Array<TestTarget>, };

export type TransferredRequest = { path: string, name: string, };

export type TransferredTrial = { test: string, step: string, };

export type TreeNode = { "kind": "folder", name: string, path: string, children: Array<TreeNode>, requestCount: number, test: boolean, } | { "kind": "request", name: string, path: string, method: HttpMethod | null, assertionCount: number, invalid: boolean, contract: string | null, url: string, tags: Array<string>, };

export type TreeView = { nodes: Array<TreeNode>, tests: Array<TreeNode>, collapsed: Array<string>, };

export type TrialResult = { snapshot: CatalogSnapshot, projectId: string, path: string, contract: string, outcome: SendOutcome, };

export type Undoable = { snapshot: CatalogSnapshot, undoToken: string, };

export type UnknownVariable = { name: string, definedIn: Array<VariableSource>, };

export type UpdateChannel = "stable" | "prerelease";

export type UpdatePrefs = { automatic: boolean, channel: UpdateChannel, lastCheck: number | null, };

export type UpdatesView = { current: string, prefs: UpdatePrefs, latest: Release | null, checked: boolean, installable: boolean, install: InstallState, };

export type UsedVariable = { name: string, value: string, runtime: boolean, };

export type VariableSource = { environment: string, value: string | null, };

export type VariableUser = { path: string, name: string, method: HttpMethod, url: string, variables: Array<string>, };

export type WindowBounds = { x: number, y: number, width: number, height: number, };

export type WindowContext = { label: string, windows: WindowsSnapshot, activeRequest: ActiveRequest | null, };

export type WindowKind = "main" | "content" | "sidebar";

export type WindowLayout = { root: LayoutNode, activeZone: string, };

export type WindowView = { label: string, kind: WindowKind, layout: WindowLayout | null, follow: boolean, sidebarWidth: number, sidebarDetached: boolean, bounds: WindowBounds | null, sidebarHidden: boolean, };

export type WindowsSnapshot = { version: number, windows: Array<WindowView>, };

export type Workspace = { id: string, name: string, color: WorkspaceColor, projectIds: Array<string>, };

export type WorkspaceColor = "gray" | "orange" | "blue" | "green" | "purple" | "red" | "yellow" | "teal";

export type WorkspaceCreated = { snapshot: CatalogSnapshot, workspaceId: string, };

export type WorkspaceView = { id: string, name: string, color: WorkspaceColor, projectIds: Array<string>, };

export type ZoneView = { id: string, tabs: Array<ContentTab>, activeTab: string | null, placement: Placement, recent: Array<string>, };

// --- Concordance avec src/types/ipc.ts (vérifiée par vue-tsc) ---

/** Vrai si chaque type est assignable à l'autre et qu'ils ont les mêmes clés. */
type Same<A, B> = [A] extends [B]
  ? [B] extends [A]
    ? [keyof A] extends [keyof B]
      ? [keyof B] extends [keyof A]
        ? true
        : false
      : false
    : false
  : false;
type Check<T extends true> = T;

export type CheckCommandErrorKind = Check<Same<Ipc.CommandErrorKind, CommandErrorKind>>;
export type CheckCommandError = Check<Same<Ipc.CommandError, CommandError>>;
export type CheckHelloResponse = Check<Same<Ipc.HelloResponse, HelloResponse>>;
export type CheckWorkspaceColor = Check<Same<Ipc.WorkspaceColor, WorkspaceColor>>;
export type CheckProjectStatus = Check<Same<Ipc.ProjectStatus, ProjectStatus>>;
export type CheckProjectView = Check<Same<Ipc.ProjectView, ProjectView>>;
export type CheckWorkspaceView = Check<Same<Ipc.WorkspaceView, WorkspaceView>>;
export type CheckCatalogSnapshot = Check<Same<Ipc.CatalogSnapshot, CatalogSnapshot>>;
export type CheckCatalogLoad = Check<Same<Ipc.CatalogLoad, CatalogLoad>>;
export type CheckUndoable = Check<Same<Ipc.Undoable, Undoable>>;
export type CheckProjectCreated = Check<Same<Ipc.ProjectCreated, ProjectCreated>>;
export type CheckWorkspaceCreated = Check<Same<Ipc.WorkspaceCreated, WorkspaceCreated>>;
export type CheckProjectLastRun = Check<Same<Ipc.ProjectLastRun, ProjectLastRun>>;
export type CheckHttpMethod = Check<Same<Ipc.HttpMethod, HttpMethod>>;
export type CheckTreeNode = Check<Same<Ipc.TreeNode, TreeNode>>;
export type CheckTreeView = Check<Same<Ipc.TreeView, TreeView>>;
export type CheckParamKind = Check<Same<Ipc.ParamKind, ParamKind>>;
export type CheckPair = Check<Same<Ipc.Pair, Pair>>;
export type CheckAuthKind = Check<Same<Ipc.AuthKind, AuthKind>>;
export type CheckApiKeyLocation = Check<Same<Ipc.ApiKeyLocation, ApiKeyLocation>>;
export type CheckAuth = Check<Same<Ipc.Auth, Auth>>;
export type CheckCredentialsPlacement = Check<Same<Ipc.CredentialsPlacement, CredentialsPlacement>>;
export type CheckTokenPlacement = Check<Same<Ipc.TokenPlacement, TokenPlacement>>;
export type CheckOAuth2 = Check<Same<Ipc.OAuth2, OAuth2>>;
export type CheckBodyKind = Check<Same<Ipc.BodyKind, BodyKind>>;
export type CheckBodyVariant = Check<Same<Ipc.BodyVariant, BodyVariant>>;
export type CheckBody = Check<Same<Ipc.Body, Body>>;
export type CheckRequestSettings = Check<Same<Ipc.RequestSettings, RequestSettings>>;
export type CheckResolvedSettings = Check<Same<Ipc.ResolvedSettings, ResolvedSettings>>;
export type CheckExampleBodyKind = Check<Same<Ipc.ExampleBodyKind, ExampleBodyKind>>;
export type CheckResponseHeader = Check<Same<Ipc.ResponseHeader, ResponseHeader>>;
export type CheckExample = Check<Same<Ipc.Example, Example>>;
export type CheckInherited = Check<Same<Ipc.Inherited, Inherited>>;
export type CheckAssertionOperator = Check<Same<Ipc.AssertionOperator, AssertionOperator>>;
export type CheckAssertionOrigin = Check<Same<Ipc.AssertionOrigin, AssertionOrigin>>;
export type CheckScalar = Check<Same<Ipc.Scalar, Scalar>>;
export type CheckAssertion = Check<Same<Ipc.Assertion, Assertion>>;
export type CheckRequestDoc = Check<Same<Ipc.RequestDoc, RequestDoc>>;
export type CheckContractLink = Check<Same<Ipc.ContractLink, ContractLink>>;
export type CheckAssertionResult = Check<Same<Ipc.AssertionResult, AssertionResult>>;
export type CheckHttpResponse = Check<Same<Ipc.HttpResponse, HttpResponse>>;
export type CheckTimeline = Check<Same<Ipc.Timeline, Timeline>>;
export type CheckSendOutcome = Check<Same<Ipc.SendOutcome, SendOutcome>>;
export type CheckEnvVariable = Check<Same<Ipc.EnvVariable, EnvVariable>>;
export type CheckEnvironmentDoc = Check<Same<Ipc.EnvironmentDoc, EnvironmentDoc>>;
export type CheckLocalVarView = Check<Same<Ipc.LocalVarView, LocalVarView>>;
export type CheckEnvironmentView = Check<Same<Ipc.EnvironmentView, EnvironmentView>>;
export type CheckEnvironmentsView = Check<Same<Ipc.EnvironmentsView, EnvironmentsView>>;
export type CheckRunItemStatus = Check<Same<Ipc.RunItemStatus, RunItemStatus>>;
export type CheckRunItem = Check<Same<Ipc.RunItem, RunItem>>;
export type CheckRunGroup = Check<Same<Ipc.RunGroup, RunGroup>>;
export type CheckRunView = Check<Same<Ipc.RunView, RunView>>;
export type CheckRequestStatus = Check<Same<Ipc.RequestStatus, RequestStatus>>;
export type CheckStatusesChange = Check<Same<Ipc.StatusesChange, StatusesChange>>;
export type CheckProposeInput = Check<Same<Ipc.ProposeInput, ProposeInput>>;
export type CheckExampleInput = Check<Same<Ipc.ExampleInput, ExampleInput>>;
export type CheckContentRef = Check<Same<Ipc.ContentRef, ContentRef>>;
export type CheckContentTab = Check<Same<Ipc.ContentTab, ContentTab>>;
export type CheckPlacement = Check<Same<Ipc.Placement, Placement>>;
export type CheckAxis = Check<Same<Ipc.Axis, Axis>>;
export type CheckSide = Check<Same<Ipc.Side, Side>>;
export type CheckLayoutPreset = Check<Same<Ipc.LayoutPreset, LayoutPreset>>;
export type CheckZoneView = Check<Same<Ipc.ZoneView, ZoneView>>;
export type CheckLayoutNode = Check<Same<Ipc.LayoutNode, LayoutNode>>;
export type CheckWindowLayout = Check<Same<Ipc.WindowLayout, WindowLayout>>;
export type CheckWindowKind = Check<Same<Ipc.WindowKind, WindowKind>>;
export type CheckWindowBounds = Check<Same<Ipc.WindowBounds, WindowBounds>>;
export type CheckWindowView = Check<Same<Ipc.WindowView, WindowView>>;
export type CheckWindowsSnapshot = Check<Same<Ipc.WindowsSnapshot, WindowsSnapshot>>;
export type CheckWindowContext = Check<Same<Ipc.WindowContext, WindowContext>>;
export type CheckActiveRequest = Check<Same<Ipc.ActiveRequest, ActiveRequest>>;
export type CheckRequestResult = Check<Same<Ipc.RequestResult, RequestResult>>;
export type CheckResultChange = Check<Same<Ipc.ResultChange, ResultChange>>;
export type CheckRunChange = Check<Same<Ipc.RunChange, RunChange>>;
export type CheckProjectScope = Check<Same<Ipc.ProjectScope, ProjectScope>>;
export type CheckProjectChange = Check<Same<Ipc.ProjectChange, ProjectChange>>;
export type CheckSpecKind = Check<Same<Ipc.SpecKind, SpecKind>>;
export type CheckContractSummary = Check<Same<Ipc.ContractSummary, ContractSummary>>;
export type CheckResourceView = Check<Same<Ipc.ResourceView, ResourceView>>;
export type CheckOperationNode = Check<Same<Ipc.OperationNode, OperationNode>>;
export type CheckLinkedView = Check<Same<Ipc.LinkedView, LinkedView>>;
export type CheckExampleRef = Check<Same<Ipc.ExampleRef, ExampleRef>>;
export type CheckOperationView = Check<Same<Ipc.OperationView, OperationView>>;
export type CheckOrphanView = Check<Same<Ipc.OrphanView, OrphanView>>;
export type CheckContractView = Check<Same<Ipc.ContractView, ContractView>>;
export type CheckContractSource = Check<Same<Ipc.ContractSource, ContractSource>>;
export type CheckSkippedOperation = Check<Same<Ipc.SkippedOperation, SkippedOperation>>;
export type CheckBaseUrlSet = Check<Same<Ipc.BaseUrlSet, BaseUrlSet>>;
export type CheckGenerateReport = Check<Same<Ipc.GenerateReport, GenerateReport>>;
export type CheckStepFamily = Check<Same<Ipc.StepFamily, StepFamily>>;
export type CheckStepFieldKind = Check<Same<Ipc.StepFieldKind, StepFieldKind>>;
export type CheckStepEngine = Check<Same<Ipc.StepEngine, StepEngine>>;
export type CheckStepSubject = Check<Same<Ipc.StepSubject, StepSubject>>;
export type CheckStepVisibleWhen = Check<Same<Ipc.StepVisibleWhen, StepVisibleWhen>>;
export type CheckStepField = Check<Same<Ipc.StepField, StepField>>;
export type CheckStepRule = Check<Same<Ipc.StepRule, StepRule>>;
export type CheckStepNature = Check<Same<Ipc.StepNature, StepNature>>;
export type CheckStepCatalogSource = Check<Same<Ipc.StepCatalogSource, StepCatalogSource>>;
export type CheckStepCatalog = Check<Same<Ipc.StepCatalog, StepCatalog>>;
export type CheckDetection = Check<Same<Ipc.Detection, Detection>>;
export type CheckTrialResult = Check<Same<Ipc.TrialResult, TrialResult>>;
export type CheckResponseFormat = Check<Same<Ipc.ResponseFormat, ResponseFormat>>;
export type CheckTokenKind = Check<Same<Ipc.TokenKind, TokenKind>>;
export type CheckBodyToken = Check<Same<Ipc.BodyToken, BodyToken>>;
export type CheckRenderedBody = Check<Same<Ipc.RenderedBody, RenderedBody>>;
export type CheckHtmlNode = Check<Same<Ipc.HtmlNode, HtmlNode>>;
export type CheckFilterMatch = Check<Same<Ipc.FilterMatch, FilterMatch>>;
export type CheckFilterResult = Check<Same<Ipc.FilterResult, FilterResult>>;
export type CheckVariableSource = Check<Same<Ipc.VariableSource, VariableSource>>;
export type CheckUnknownVariable = Check<Same<Ipc.UnknownVariable, UnknownVariable>>;
export type CheckAddedOrigin = Check<Same<Ipc.AddedOrigin, AddedOrigin>>;
export type CheckAddedHeader = Check<Same<Ipc.AddedHeader, AddedHeader>>;
export type CheckRequestInput = Check<Same<Ipc.RequestInput, RequestInput>>;
export type CheckCurlCopy = Check<Same<Ipc.CurlCopy, CurlCopy>>;
export type CheckPairsFormat = Check<Same<Ipc.PairsFormat, PairsFormat>>;
export type CheckTextError = Check<Same<Ipc.TextError, TextError>>;
export type CheckPairsText = Check<Same<Ipc.PairsText, PairsText>>;
export type CheckCurlSecret = Check<Same<Ipc.CurlSecret, CurlSecret>>;
export type CheckCurlIgnored = Check<Same<Ipc.CurlIgnored, CurlIgnored>>;
export type CheckCurlImport = Check<Same<Ipc.CurlImport, CurlImport>>;
export type CheckImportFormat = Check<Same<Ipc.ImportFormat, ImportFormat>>;
export type CheckImportSource = Check<Same<Ipc.ImportSource, ImportSource>>;
export type CheckImportTarget = Check<Same<Ipc.ImportTarget, ImportTarget>>;
export type CheckImportItemKind = Check<Same<Ipc.ImportItemKind, ImportItemKind>>;
export type CheckImportItemStatus = Check<Same<Ipc.ImportItemStatus, ImportItemStatus>>;
export type CheckImportReportItem = Check<Same<Ipc.ImportReportItem, ImportReportItem>>;
export type CheckConversionExample = Check<Same<Ipc.ConversionExample, ConversionExample>>;
export type CheckImportCounts = Check<Same<Ipc.ImportCounts, ImportCounts>>;
export type CheckImportReport = Check<Same<Ipc.ImportReport, ImportReport>>;
export type CheckImportDone = Check<Same<Ipc.ImportDone, ImportDone>>;
export type CheckExtraction = Check<Same<Ipc.Extraction, Extraction>>;
export type CheckExtractedValue = Check<Same<Ipc.ExtractedValue, ExtractedValue>>;
export type CheckDrawnValue = Check<Same<Ipc.DrawnValue, DrawnValue>>;
export type CheckUsedVariable = Check<Same<Ipc.UsedVariable, UsedVariable>>;
export type CheckSent = Check<Same<Ipc.Sent, Sent>>;
export type CheckRunTest = Check<Same<Ipc.RunTest, RunTest>>;
export type CheckStepView = Check<Same<Ipc.StepView, StepView>>;
export type CheckTestView = Check<Same<Ipc.TestView, TestView>>;
export type CheckCreatedTest = Check<Same<Ipc.CreatedTest, CreatedTest>>;
export type CheckVariableUser = Check<Same<Ipc.VariableUser, VariableUser>>;
export type CheckPathParam = Check<Same<Ipc.PathParam, PathParam>>;
export type CheckPathGeneralization = Check<Same<Ipc.PathGeneralization, PathGeneralization>>;
export type CheckTransferredTrial = Check<Same<Ipc.TransferredTrial, TransferredTrial>>;
export type CheckFiledTrials = Check<Same<Ipc.FiledTrials, FiledTrials>>;
export type CheckDynamicValue = Check<Same<Ipc.DynamicValue, DynamicValue>>;
export type CheckProxyMode = Check<Same<Ipc.ProxyMode, ProxyMode>>;
export type CheckProxySource = Check<Same<Ipc.ProxySource, ProxySource>>;
export type CheckManualProxy = Check<Same<Ipc.ManualProxy, ManualProxy>>;
export type CheckProxyPrefs = Check<Same<Ipc.ProxyPrefs, ProxyPrefs>>;
export type CheckSystemProxy = Check<Same<Ipc.SystemProxy, SystemProxy>>;
export type CheckCertificateInfo = Check<Same<Ipc.CertificateInfo, CertificateInfo>>;
export type CheckAuthorityView = Check<Same<Ipc.AuthorityView, AuthorityView>>;
export type CheckNetworkView = Check<Same<Ipc.NetworkView, NetworkView>>;
export type CheckNetworkPrefsInput = Check<Same<Ipc.NetworkPrefsInput, NetworkPrefsInput>>;
export type CheckFailureCause = Check<Same<Ipc.FailureCause, FailureCause>>;
export type CheckCertificateProblem = Check<Same<Ipc.CertificateProblem, CertificateProblem>>;
export type CheckCertificateFailure = Check<Same<Ipc.CertificateFailure, CertificateFailure>>;
export type CheckNetworkFailure = Check<Same<Ipc.NetworkFailure, NetworkFailure>>;
export type CheckTokenStatus = Check<Same<Ipc.TokenStatus, TokenStatus>>;
export type CheckAuthOverride = Check<Same<Ipc.AuthOverride, AuthOverride>>;
export type CheckAuthUsage = Check<Same<Ipc.AuthUsage, AuthUsage>>;
export type CheckProjectAuthView = Check<Same<Ipc.ProjectAuthView, ProjectAuthView>>;
export type CheckCookieView = Check<Same<Ipc.CookieView, CookieView>>;
export type CheckCookieDomain = Check<Same<Ipc.CookieDomain, CookieDomain>>;
export type CheckCookieJarView = Check<Same<Ipc.CookieJarView, CookieJarView>>;
export type CheckMenuShortcut = Check<Same<Ipc.MenuShortcut, MenuShortcut>>;
export type CheckRenamedFile = Check<Same<Ipc.RenamedFile, RenamedFile>>;
export type CheckFilesChange = Check<Same<Ipc.FilesChange, FilesChange>>;
export type CheckConflictSide = Check<Same<Ipc.ConflictSide, ConflictSide>>;
export type CheckConflictItem = Check<Same<Ipc.ConflictItem, ConflictItem>>;
export type CheckRequestConflict = Check<Same<Ipc.RequestConflict, RequestConflict>>;
export type CheckProblemNature = Check<Same<Ipc.ProblemNature, ProblemNature>>;
export type CheckStartupAction = Check<Same<Ipc.StartupAction, StartupAction>>;
export type CheckStartupTarget = Check<Same<Ipc.StartupTarget, StartupTarget>>;
export type CheckStartupOption = Check<Same<Ipc.StartupOption, StartupOption>>;
export type CheckRetryPlan = Check<Same<Ipc.RetryPlan, RetryPlan>>;
export type CheckDetailRow = Check<Same<Ipc.DetailRow, DetailRow>>;
export type CheckStartupProblem = Check<Same<Ipc.StartupProblem, StartupProblem>>;
export type CheckTransferMode = Check<Same<Ipc.TransferMode, TransferMode>>;
export type CheckTransferTarget = Check<Same<Ipc.TransferTarget, TransferTarget>>;
export type CheckOperationTarget = Check<Same<Ipc.OperationTarget, OperationTarget>>;
export type CheckFolderTarget = Check<Same<Ipc.FolderTarget, FolderTarget>>;
export type CheckTestTarget = Check<Same<Ipc.TestTarget, TestTarget>>;
export type CheckTransferTargets = Check<Same<Ipc.TransferTargets, TransferTargets>>;
export type CheckTransferChange = Check<Same<Ipc.TransferChange, TransferChange>>;
export type CheckTransferDone = Check<Same<Ipc.TransferDone, TransferDone>>;
export type CheckUpdateChannel = Check<Same<Ipc.UpdateChannel, UpdateChannel>>;
export type CheckUpdatePrefs = Check<Same<Ipc.UpdatePrefs, UpdatePrefs>>;
export type CheckRelease = Check<Same<Ipc.Release, Release>>;
export type CheckUpdatesView = Check<Same<Ipc.UpdatesView, UpdatesView>>;
export type CheckClearSecret = Check<Same<Ipc.ClearSecret, ClearSecret>>;
export type CheckBatchAction = Check<Same<Ipc.BatchAction, BatchAction>>;
export type CheckBatchResult = Check<Same<Ipc.BatchResult, BatchResult>>;
export type CheckInstallState = Check<Same<Ipc.InstallState, InstallState>>;

// Sans homologue généré (écrits seulement côté interface) :
// - CommandName
// - ProjectCreateInput
// - EnvironmentColor
// - RequestStatuses
