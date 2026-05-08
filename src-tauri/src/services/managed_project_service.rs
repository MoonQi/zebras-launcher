use crate::models::{
    CreateProjectInstanceInput, ManagedFrontendLink, ManagedProjectManifest, ManagedRepo,
    ManagedRepoInput, ProjectInfo, ProjectSourceType, ProvisionStatus, RepoRole, ValidationResult,
    Workspace, WorkspaceSourceType, ZebrasVersion,
};
use crate::services::config_parser::ConfigParser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(not(target_os = "windows"))]
use crate::utils::{resolve_program_in_user_path, USER_PATH};

const MANIFEST_FILENAME: &str = ".zebras-project.json";

pub struct ManagedProjectService;

impl ManagedProjectService {
    pub fn manifest_path(root_path: &Path) -> PathBuf {
        root_path.join(MANIFEST_FILENAME)
    }

    pub fn validate_input(input: &CreateProjectInstanceInput) -> ValidationResult {
        let mut errors = Vec::new();
        let root_path = PathBuf::from(input.root_path.trim());

        if input.project_name.trim().is_empty() {
            errors.push("项目名称不能为空".to_string());
        }

        if input.root_path.trim().is_empty() {
            errors.push("本地根目录不能为空".to_string());
        } else if root_path.exists() {
            if !root_path.is_dir() {
                errors.push("本地根目录必须是目录".to_string());
            } else if fs::read_dir(&root_path)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false)
            {
                errors.push("本地根目录必须为空目录".to_string());
            }
        }

        if input.repos.is_empty() {
            errors.push("至少需要配置一个仓库".to_string());
        }

        let mut repo_ids = HashSet::new();
        let mut mount_paths = HashSet::new();
        let normalized_repos: Vec<ManagedRepoInput> =
            input.repos.iter().map(Self::normalize_repo_input).collect();
        let repo_by_id: HashMap<&str, &ManagedRepoInput> = normalized_repos
            .iter()
            .map(|repo| (repo.id.as_str(), repo))
            .collect();

        for repo in &normalized_repos {
            if repo.id.trim().is_empty() {
                errors.push("仓库 ID 不能为空".to_string());
            } else if !is_valid_repo_id(&repo.id) {
                errors.push(format!(
                    "仓库 ID `{}` 只允许字母、数字、点、下划线和中划线",
                    repo.id
                ));
            } else if !repo_ids.insert(repo.id.clone()) {
                errors.push(format!("仓库 ID `{}` 重复", repo.id));
            }

            if repo.display_name.trim().is_empty() {
                errors.push(format!("仓库 `{}` 的显示名不能为空", repo.id));
            }

            if repo.git_url.trim().is_empty() {
                errors.push(format!("仓库 `{}` 的 Git 地址不能为空", repo.id));
            }

            let mount_path = mount_path_for(&repo.role, &repo.id);
            if !mount_paths.insert(mount_path.clone()) {
                errors.push(format!("挂载路径 `{}` 重复", mount_path));
            }
        }

        let mut seen_links = HashSet::new();
        for link in &input.frontend_links {
            let edge_key = format!("{}->{}", link.provider_repo_id, link.consumer_repo_id);
            if !seen_links.insert(edge_key.clone()) {
                errors.push(format!("前端链接 `{}` 重复", edge_key));
            }

            let Some(provider) = repo_by_id.get(link.provider_repo_id.as_str()) else {
                errors.push(format!("链接 provider `{}` 不存在", link.provider_repo_id));
                continue;
            };
            let Some(consumer) = repo_by_id.get(link.consumer_repo_id.as_str()) else {
                errors.push(format!("链接 consumer `{}` 不存在", link.consumer_repo_id));
                continue;
            };

            if provider.role != RepoRole::FrontendPackage {
                errors.push(format!(
                    "链接 provider `{}` 必须是 frontend_package",
                    link.provider_repo_id
                ));
            }
            if consumer.role != RepoRole::FrontendApp {
                errors.push(format!(
                    "链接 consumer `{}` 必须是 frontend_app",
                    link.consumer_repo_id
                ));
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
        }
    }

    pub fn build_manifest(input: &CreateProjectInstanceInput) -> ManagedProjectManifest {
        ManagedProjectManifest {
            version: 1,
            project_name: input.project_name.trim().to_string(),
            root_path: PathBuf::from(input.root_path.trim()),
            status: ProvisionStatus::Pending,
            repos: input
                .repos
                .iter()
                .map(Self::normalize_repo_input)
                .map(|repo| {
                    let mount_path = mount_path_for(&repo.role, &repo.id);
                    ManagedRepo {
                        id: repo.id,
                        display_name: repo.display_name,
                        role: repo.role.clone(),
                        git_url: repo.git_url,
                        branch: repo.branch,
                        mount_path,
                        status: ProvisionStatus::Pending,
                        last_error: None,
                    }
                })
                .collect(),
            frontend_links: input
                .frontend_links
                .iter()
                .map(|link| ManagedFrontendLink {
                    provider_repo_id: link.provider_repo_id.clone(),
                    consumer_repo_id: link.consumer_repo_id.clone(),
                    status: ProvisionStatus::Pending,
                    last_error: None,
                })
                .collect(),
        }
    }

    pub fn save_manifest(manifest: &ManagedProjectManifest) -> Result<(), String> {
        let path = Self::manifest_path(&manifest.root_path);
        let content = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("序列化项目清单失败: {}", e))?;
        fs::write(path, content).map_err(|e| format!("写入项目清单失败: {}", e))
    }

    pub fn load_manifest(root_path: &Path) -> Result<ManagedProjectManifest, String> {
        let path = Self::manifest_path(root_path);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取项目清单失败 ({}): {}", path.display(), e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析项目清单失败: {}", e))
    }

    pub fn create_project_instance(input: CreateProjectInstanceInput) -> Result<Workspace, String> {
        let validation = Self::validate_input(&input);
        if !validation.valid {
            return Err(validation.errors.join("\n"));
        }

        let mut manifest = Self::build_manifest(&input);
        fs::create_dir_all(&manifest.root_path)
            .map_err(|e| format!("创建项目根目录失败: {}", e))?;
        Self::ensure_runtime_dirs(&manifest.root_path)?;

        manifest.status = ProvisionStatus::Provisioning;
        Self::save_manifest(&manifest)?;

        for index in 0..manifest.repos.len() {
            Self::provision_repo(&mut manifest, index)?;
        }

        Self::provision_links(&mut manifest, true)?;
        Self::finalize_manifest(&mut manifest);
        Self::save_manifest(&manifest)?;

        Ok(Self::workspace_from_manifest(&manifest, None))
    }

    pub fn repair_project_instance(
        root_path: &Path,
        base: Option<&Workspace>,
    ) -> Result<Workspace, String> {
        let mut manifest = Self::load_manifest(root_path)?;
        Self::ensure_runtime_dirs(&manifest.root_path)?;
        manifest.status = ProvisionStatus::Provisioning;
        Self::save_manifest(&manifest)?;

        let mut retried_links = false;
        for index in 0..manifest.repos.len() {
            let repo_path = manifest.root_path.join(&manifest.repos[index].mount_path);
            let should_retry =
                manifest.repos[index].status != ProvisionStatus::Ready || !repo_path.exists();
            if should_retry {
                Self::provision_repo(&mut manifest, index)?;
                retried_links = true;
            }
        }

        if retried_links
            || manifest
                .frontend_links
                .iter()
                .any(|link| link.status != ProvisionStatus::Ready)
        {
            Self::provision_links(&mut manifest, false)?;
        }

        Self::finalize_manifest(&mut manifest);
        Self::save_manifest(&manifest)?;
        Ok(Self::workspace_from_manifest(&manifest, base))
    }

    pub fn rebuild_project_links(
        root_path: &Path,
        base: Option<&Workspace>,
    ) -> Result<Workspace, String> {
        let mut manifest = Self::load_manifest(root_path)?;
        manifest.status = ProvisionStatus::Provisioning;
        for link in manifest.frontend_links.iter_mut() {
            link.status = ProvisionStatus::Pending;
            link.last_error = None;
        }
        Self::save_manifest(&manifest)?;
        Self::provision_links(&mut manifest, false)?;
        Self::finalize_manifest(&mut manifest);
        Self::save_manifest(&manifest)?;
        Ok(Self::workspace_from_manifest(&manifest, base))
    }

    pub fn workspace_from_manifest(
        manifest: &ManagedProjectManifest,
        base: Option<&Workspace>,
    ) -> Workspace {
        let mut workspace = base.cloned().unwrap_or_else(|| {
            Workspace::new(manifest.project_name.clone(), manifest.root_path.clone())
        });
        workspace.name = manifest.project_name.clone();
        workspace.root_path = manifest.root_path.clone();
        workspace.source_type = WorkspaceSourceType::ManagedProject;
        workspace.provision_status = Some(manifest.status.clone());
        workspace.folders = vec![manifest.root_path.to_string_lossy().to_string()];
        workspace.projects = manifest
            .repos
            .iter()
            .map(|repo| Self::project_from_repo(manifest, repo))
            .collect();
        workspace.last_modified = chrono::Utc::now();
        workspace
    }

    pub fn refresh_workspace(workspace: &Workspace) -> Result<Workspace, String> {
        let manifest = Self::load_manifest(&workspace.root_path)?;
        Ok(Self::workspace_from_manifest(&manifest, Some(workspace)))
    }

    fn provision_repo(manifest: &mut ManagedProjectManifest, index: usize) -> Result<(), String> {
        manifest.repos[index].status = ProvisionStatus::Provisioning;
        manifest.repos[index].last_error = None;
        Self::save_manifest(manifest)?;

        let repo_path = manifest.root_path.join(&manifest.repos[index].mount_path);
        let result = (|| {
            if !repo_path.exists() {
                if let Some(parent) = repo_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("创建仓库父目录失败: {}", e))?;
                }
                Self::git_clone(
                    &manifest.repos[index].git_url,
                    manifest.repos[index].branch.as_deref(),
                    &repo_path,
                )?;
            }

            Self::validate_repo_contents(&manifest.repos[index].role, &repo_path)
        })();

        match result {
            Ok(()) => {
                manifest.repos[index].status = ProvisionStatus::Ready;
                manifest.repos[index].last_error = None;
            }
            Err(err) => {
                manifest.repos[index].status = ProvisionStatus::Degraded;
                manifest.repos[index].last_error = Some(err);
            }
        }

        Self::save_manifest(manifest)
    }

    fn provision_links(
        manifest: &mut ManagedProjectManifest,
        reset_ready: bool,
    ) -> Result<(), String> {
        for index in 0..manifest.frontend_links.len() {
            if !reset_ready && manifest.frontend_links[index].status == ProvisionStatus::Ready {
                continue;
            }

            manifest.frontend_links[index].status = ProvisionStatus::Provisioning;
            manifest.frontend_links[index].last_error = None;
            Self::save_manifest(manifest)?;

            let result = Self::apply_link(manifest, index);
            match result {
                Ok(()) => {
                    manifest.frontend_links[index].status = ProvisionStatus::Ready;
                    manifest.frontend_links[index].last_error = None;
                }
                Err(err) => {
                    manifest.frontend_links[index].status = ProvisionStatus::Degraded;
                    manifest.frontend_links[index].last_error = Some(err);
                }
            }

            Self::save_manifest(manifest)?;
        }

        Ok(())
    }

    fn apply_link(manifest: &ManagedProjectManifest, index: usize) -> Result<(), String> {
        let link = &manifest.frontend_links[index];
        let repo_by_id: HashMap<&str, &ManagedRepo> = manifest
            .repos
            .iter()
            .map(|repo| (repo.id.as_str(), repo))
            .collect();

        let provider = repo_by_id
            .get(link.provider_repo_id.as_str())
            .ok_or_else(|| format!("provider `{}` 不存在", link.provider_repo_id))?;
        let consumer = repo_by_id
            .get(link.consumer_repo_id.as_str())
            .ok_or_else(|| format!("consumer `{}` 不存在", link.consumer_repo_id))?;

        if provider.role != RepoRole::FrontendPackage {
            return Err(format!("provider `{}` 不是 frontend_package", provider.id));
        }
        if consumer.role != RepoRole::FrontendApp {
            return Err(format!("consumer `{}` 不是 frontend_app", consumer.id));
        }
        if provider.status != ProvisionStatus::Ready {
            return Err(format!("provider `{}` 尚未就绪", provider.id));
        }
        if consumer.status != ProvisionStatus::Ready {
            return Err(format!("consumer `{}` 尚未就绪", consumer.id));
        }

        let provider_path = manifest.root_path.join(&provider.mount_path);
        let consumer_path = manifest.root_path.join(&consumer.mount_path);
        let package_name = read_package_name(&provider_path)?;
        ensure_dependency_declared(&consumer_path, &package_name)?;

        Self::run_command("npm", &["link"], &provider_path)?;
        Self::run_command("npm", &["link", &package_name], &consumer_path)?;

        let links_dir = manifest.root_path.join(".zebras").join("links");
        fs::create_dir_all(&links_dir).map_err(|e| format!("创建 links 目录失败: {}", e))?;
        let record_name = format!("{}__{}.json", provider.id, consumer.id);
        let record = serde_json::json!({
            "providerRepoId": provider.id,
            "consumerRepoId": consumer.id,
            "packageName": package_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(
            links_dir.join(record_name),
            serde_json::to_string_pretty(&record)
                .map_err(|e| format!("序列化 link 记录失败: {}", e))?,
        )
        .map_err(|e| format!("写入 link 记录失败: {}", e))?;

        Ok(())
    }

    fn project_from_repo(manifest: &ManagedProjectManifest, repo: &ManagedRepo) -> ProjectInfo {
        let repo_path = manifest.root_path.join(&repo.mount_path);
        let mut project = ProjectInfo::new(repo_path.clone(), repo.display_name.clone());
        project.id = repo.id.clone();
        project.version = ZebrasVersion::Managed;
        project.source_type = ProjectSourceType::ManagedProject;
        project.repo_role = Some(repo.role.clone());
        project.provision_status = Some(repo.status.clone());
        project.runnable = false;
        project.enabled = Some(false);
        project.type_ = managed_type_for_role(&repo.role);
        project.platform = managed_platform_for_role(&repo.role);
        project.port = 0;
        project.debug = None;
        project.error = repo.last_error.clone();
        project.is_valid = repo_path.exists();

        if repo_path.exists() {
            project.framework = detect_framework(&repo.role, &repo_path);
            if let Ok(parsed) = ConfigParser::parse_project(&repo_path) {
                if repo.role == RepoRole::FrontendApp {
                    project.version = parsed.version;
                    project.platform = parsed.platform;
                    project.type_ = parsed.type_;
                    project.debug = parsed.debug;
                    project.runnable = parsed.is_valid;
                    project.enabled = Some(parsed.is_valid);
                }
                if parsed.framework.is_some() {
                    project.framework = parsed.framework;
                }
                project.domain = parsed.domain;
                project.port = parsed.port;
            }
        } else {
            project.framework = detect_framework(&repo.role, &repo_path);
        }

        project
    }

    fn ensure_runtime_dirs(root_path: &Path) -> Result<(), String> {
        for dir in [
            root_path.join(".zebras"),
            root_path.join(".zebras").join("cache"),
            root_path.join(".zebras").join("logs"),
            root_path.join(".zebras").join("links"),
        ] {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("创建运行目录失败 ({}): {}", dir.display(), e))?;
        }

        Ok(())
    }

    fn validate_repo_contents(role: &RepoRole, repo_path: &Path) -> Result<(), String> {
        if !repo_path.exists() {
            return Err("仓库目录不存在".to_string());
        }

        match role {
            RepoRole::FrontendApp | RepoRole::FrontendPackage => {
                let package_json = repo_path.join("package.json");
                if !package_json.exists() {
                    return Err("未检测到 package.json，角色与仓库内容不匹配".to_string());
                }
                let _ = read_package_json(repo_path)?;
            }
            RepoRole::BackendService => {
                let has_java_build = ["pom.xml", "build.gradle", "build.gradle.kts"]
                    .iter()
                    .any(|file| repo_path.join(file).exists());
                if !has_java_build {
                    return Err("未检测到 Maven/Gradle 构建文件，角色与仓库内容不匹配".to_string());
                }
            }
        }

        Ok(())
    }

    fn git_clone(git_url: &str, branch: Option<&str>, destination: &Path) -> Result<(), String> {
        let mut args = vec!["clone".to_string(), "--depth".to_string(), "1".to_string()];
        if let Some(branch) = branch.filter(|branch| !branch.trim().is_empty()) {
            args.push("--branch".to_string());
            args.push(branch.to_string());
        }
        args.push(git_url.to_string());
        args.push(destination.to_string_lossy().to_string());

        let destination_parent = destination
            .parent()
            .ok_or_else(|| "无法确定 clone 目标父目录".to_string())?;
        Self::run_command_owned("git", &args, destination_parent)
    }

    fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
        let owned_args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        Self::run_command_owned(program, &owned_args, cwd)
    }

    fn run_command_owned(program: &str, args: &[String], cwd: &Path) -> Result<(), String> {
        let resolved_program = resolve_program(program);
        let mut cmd = Command::new(&resolved_program);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd.env("PATH", &*USER_PATH);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                format!("未找到命令 `{}`", program)
            } else {
                format!("执行 `{}` 失败: {}", program, e)
            }
        })?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stderr.is_empty() {
            Err(stderr)
        } else if !stdout.is_empty() {
            Err(stdout)
        } else {
            Err(format!(
                "命令 `{}` 退出码 {}",
                program,
                output.status.code().unwrap_or(-1)
            ))
        }
    }

    fn finalize_manifest(manifest: &mut ManagedProjectManifest) {
        let repo_degraded = manifest
            .repos
            .iter()
            .any(|repo| repo.status == ProvisionStatus::Degraded);
        let link_degraded = manifest
            .frontend_links
            .iter()
            .any(|link| link.status == ProvisionStatus::Degraded);
        manifest.status = if repo_degraded || link_degraded {
            ProvisionStatus::Degraded
        } else {
            ProvisionStatus::Ready
        };
    }

    fn normalize_repo_input(repo: &ManagedRepoInput) -> ManagedRepoInput {
        let derived_name = derive_repo_name(&repo.git_url);
        let normalized_id = if repo.id.trim().is_empty() {
            derived_name.clone().unwrap_or_default()
        } else {
            repo.id.trim().to_string()
        };
        let normalized_display_name = if repo.display_name.trim().is_empty() {
            derived_name.unwrap_or_else(|| normalized_id.clone())
        } else {
            repo.display_name.trim().to_string()
        };

        ManagedRepoInput {
            id: normalized_id,
            display_name: normalized_display_name,
            role: repo.role.clone(),
            git_url: repo.git_url.trim().to_string(),
            branch: repo
                .branch
                .clone()
                .filter(|branch| !branch.trim().is_empty()),
        }
    }
}

fn resolve_program(program: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        match program {
            "npm" => "npm.cmd".to_string(),
            other => other.to_string(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        resolve_program_in_user_path(program).unwrap_or_else(|| program.to_string())
    }
}

fn is_valid_repo_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn derive_repo_name(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    trimmed
        .rsplit(['/', ':'])
        .find(|segment| !segment.is_empty())
        .map(|segment| segment.trim_end_matches(".git").to_string())
        .filter(|segment| !segment.is_empty())
}

fn mount_path_for(role: &RepoRole, repo_id: &str) -> String {
    match role {
        RepoRole::FrontendApp => format!("apps/{}", repo_id),
        RepoRole::BackendService => format!("services/{}", repo_id),
        RepoRole::FrontendPackage => format!("packages/{}", repo_id),
    }
}

fn managed_type_for_role(role: &RepoRole) -> String {
    match role {
        RepoRole::FrontendApp => "frontend_app".to_string(),
        RepoRole::BackendService => "backend_service".to_string(),
        RepoRole::FrontendPackage => "frontend_package".to_string(),
    }
}

fn managed_platform_for_role(role: &RepoRole) -> String {
    match role {
        RepoRole::FrontendApp | RepoRole::FrontendPackage => "frontend".to_string(),
        RepoRole::BackendService => "backend".to_string(),
    }
}

fn detect_framework(role: &RepoRole, repo_path: &Path) -> Option<String> {
    match role {
        RepoRole::BackendService => Some("Java".to_string()),
        RepoRole::FrontendApp | RepoRole::FrontendPackage => {
            let package_json = read_package_json(repo_path).ok()?;
            let deps = package_json
                .get("dependencies")
                .and_then(|value| value.as_object());
            let dev_deps = package_json
                .get("devDependencies")
                .and_then(|value| value.as_object());
            if deps.is_some_and(|map| map.contains_key("react"))
                || dev_deps.is_some_and(|map| map.contains_key("react"))
            {
                Some("React".to_string())
            } else {
                Some("Node".to_string())
            }
        }
    }
}

fn read_package_json(repo_path: &Path) -> Result<serde_json::Value, String> {
    let package_path = repo_path.join("package.json");
    let content = fs::read_to_string(&package_path)
        .map_err(|e| format!("读取 package.json 失败 ({}): {}", package_path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("解析 package.json 失败: {}", e))
}

fn read_package_name(repo_path: &Path) -> Result<String, String> {
    let package_json = read_package_json(repo_path)?;
    package_json
        .get("name")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "provider package.json 缺少 name 字段".to_string())
}

fn ensure_dependency_declared(repo_path: &Path, package_name: &str) -> Result<(), String> {
    let package_json = read_package_json(repo_path)?;
    let sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];
    let declared = sections.iter().any(|section| {
        package_json
            .get(*section)
            .and_then(|value| value.as_object())
            .is_some_and(|deps| deps.contains_key(package_name))
    });

    if declared {
        Ok(())
    } else {
        Err(format!(
            "consumer 未声明依赖 `{}`，跳过 link 以避免写入 package.json",
            package_name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateProjectInstanceInput, ManagedFrontendLinkInput, ManagedRepoInput};
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zebras-launcher-managed-project-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ));
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        dir
    }

    fn command_exists(program: &str) -> bool {
        let resolved = resolve_program(program);
        Command::new(resolved)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn init_git_repo(path: &Path, files: &[(&str, &str)]) {
        fs::create_dir_all(path).unwrap();
        run_test_cmd("git", &["init"], path);
        run_test_cmd("git", &["config", "user.email", "codex@example.com"], path);
        run_test_cmd("git", &["config", "user.name", "Codex"], path);
        for (file, content) in files {
            let target = path.join(file);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(target, content).unwrap();
        }
        run_test_cmd("git", &["add", "."], path);
        run_test_cmd("git", &["commit", "-m", "init"], path);
    }

    fn run_test_cmd(program: &str, args: &[&str], cwd: &Path) {
        let resolved = resolve_program(program);
        let status = Command::new(resolved)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "command failed: {} {:?}", program, args);
    }

    #[test]
    fn validate_rejects_non_empty_root_and_invalid_links() {
        let root = temp_path("validation-root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "occupied").unwrap();

        let input = CreateProjectInstanceInput {
            project_name: "demo".to_string(),
            root_path: root.to_string_lossy().to_string(),
            repos: vec![
                ManagedRepoInput {
                    id: "admin".to_string(),
                    display_name: "Admin".to_string(),
                    role: RepoRole::FrontendApp,
                    git_url: "git@example.com:admin.git".to_string(),
                    branch: None,
                },
                ManagedRepoInput {
                    id: "admin".to_string(),
                    display_name: "Admin Duplicate".to_string(),
                    role: RepoRole::BackendService,
                    git_url: "git@example.com:svc.git".to_string(),
                    branch: None,
                },
            ],
            frontend_links: vec![ManagedFrontendLinkInput {
                provider_repo_id: "admin".to_string(),
                consumer_repo_id: "missing".to_string(),
            }],
        };

        let result = ManagedProjectService::validate_input(&input);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|err| err.contains("为空目录")));
        assert!(result.errors.iter().any(|err| err.contains("重复")));
        assert!(result.errors.iter().any(|err| err.contains("consumer")));
    }

    #[test]
    fn create_project_instance_clones_local_repos_and_loads_manifest() {
        if !command_exists("git") {
            return;
        }

        let source_root = temp_path("source");
        let frontend_repo = source_root.join("admin-web");
        let backend_repo = source_root.join("user-service");
        init_git_repo(
            &frontend_repo,
            &[
                (
                    "package.json",
                    r#"{"name":"admin-web","private":true,"scripts":{"start":"vite"},"dependencies":{"react":"18.0.0"}}"#,
                ),
                (
                    "zebras.config.ts",
                    r#"export default {
  name: "admin-web",
  platform: "web",
  type: "app",
  port: 3100,
  framework: "react"
}"#,
                ),
            ],
        );
        init_git_repo(&backend_repo, &[("pom.xml", "<project></project>")]);

        let target_root = temp_path("target");
        let input = CreateProjectInstanceInput {
            project_name: "demo".to_string(),
            root_path: target_root.to_string_lossy().to_string(),
            repos: vec![
                ManagedRepoInput {
                    id: "admin-web".to_string(),
                    display_name: "Admin Web".to_string(),
                    role: RepoRole::FrontendApp,
                    git_url: frontend_repo.to_string_lossy().to_string(),
                    branch: None,
                },
                ManagedRepoInput {
                    id: "user-service".to_string(),
                    display_name: "User Service".to_string(),
                    role: RepoRole::BackendService,
                    git_url: backend_repo.to_string_lossy().to_string(),
                    branch: None,
                },
            ],
            frontend_links: vec![],
        };

        let workspace = ManagedProjectService::create_project_instance(input).unwrap();
        assert_eq!(workspace.source_type, WorkspaceSourceType::ManagedProject);
        assert_eq!(workspace.projects.len(), 2);
        assert_eq!(workspace.provision_status, Some(ProvisionStatus::Ready));
        assert!(workspace.root_path.join("apps/admin-web").exists());
        assert!(workspace.root_path.join("services/user-service").exists());
        assert!(!workspace.root_path.join("packages").exists());
        let frontend_project = workspace
            .projects
            .iter()
            .find(|project| project.id == "admin-web")
            .unwrap();
        assert!(frontend_project.runnable);
        assert_eq!(frontend_project.version, ZebrasVersion::V3);
        assert_eq!(frontend_project.port, 3100);

        let manifest = ManagedProjectService::load_manifest(&workspace.root_path).unwrap();
        assert_eq!(manifest.status, ProvisionStatus::Ready);
        assert_eq!(manifest.repos.len(), 2);
    }

    #[test]
    fn create_project_instance_marks_failed_clone_as_degraded() {
        if !command_exists("git") {
            return;
        }

        let target_root = temp_path("degraded-target");
        let input = CreateProjectInstanceInput {
            project_name: "demo".to_string(),
            root_path: target_root.to_string_lossy().to_string(),
            repos: vec![ManagedRepoInput {
                id: "missing-service".to_string(),
                display_name: "Missing Service".to_string(),
                role: RepoRole::BackendService,
                git_url: target_root
                    .join("missing-repo")
                    .to_string_lossy()
                    .to_string(),
                branch: None,
            }],
            frontend_links: vec![],
        };

        let workspace = ManagedProjectService::create_project_instance(input).unwrap();
        assert_eq!(workspace.provision_status, Some(ProvisionStatus::Degraded));
        assert_eq!(workspace.projects.len(), 1);
        assert_eq!(
            workspace.projects[0].provision_status,
            Some(ProvisionStatus::Degraded)
        );
    }

    #[test]
    fn create_project_instance_links_frontend_package_when_declared() {
        if !command_exists("git") || !command_exists("npm") {
            return;
        }

        let source_root = temp_path("source-link");
        let provider_repo = source_root.join("ui-kit");
        let consumer_repo = source_root.join("admin-web");
        init_git_repo(
            &provider_repo,
            &[
                (
                    "package.json",
                    r#"{"name":"@acme/ui-kit","version":"1.0.0","main":"index.js"}"#,
                ),
                ("index.js", "module.exports = {};"),
            ],
        );
        init_git_repo(
            &consumer_repo,
            &[(
                "package.json",
                r#"{"name":"admin-web","private":true,"dependencies":{"@acme/ui-kit":"*","react":"18.0.0"}}"#,
            )],
        );

        let target_root = temp_path("target-link");
        let input = CreateProjectInstanceInput {
            project_name: "demo-link".to_string(),
            root_path: target_root.to_string_lossy().to_string(),
            repos: vec![
                ManagedRepoInput {
                    id: "ui-kit".to_string(),
                    display_name: "UI Kit".to_string(),
                    role: RepoRole::FrontendPackage,
                    git_url: provider_repo.to_string_lossy().to_string(),
                    branch: None,
                },
                ManagedRepoInput {
                    id: "admin-web".to_string(),
                    display_name: "Admin Web".to_string(),
                    role: RepoRole::FrontendApp,
                    git_url: consumer_repo.to_string_lossy().to_string(),
                    branch: None,
                },
            ],
            frontend_links: vec![ManagedFrontendLinkInput {
                provider_repo_id: "ui-kit".to_string(),
                consumer_repo_id: "admin-web".to_string(),
            }],
        };

        let workspace = ManagedProjectService::create_project_instance(input).unwrap();
        assert_eq!(workspace.provision_status, Some(ProvisionStatus::Ready));
        let record = workspace
            .root_path
            .join(".zebras")
            .join("links")
            .join("ui-kit__admin-web.json");
        assert!(record.exists());
    }

    #[test]
    fn normalize_repo_input_derives_id_and_display_name_from_git_url() {
        let input = ManagedRepoInput {
            id: "".to_string(),
            display_name: "".to_string(),
            role: RepoRole::FrontendApp,
            git_url: "https://hyperv28.msdi.cn/tfs/KeyTechnology/publicValueComponent/_git/report_saas_zebras_main".to_string(),
            branch: None,
        };

        let normalized = ManagedProjectService::normalize_repo_input(&input);
        assert_eq!(normalized.id, "report_saas_zebras_main");
        assert_eq!(normalized.display_name, "report_saas_zebras_main");
    }

    #[test]
    fn derive_repo_name_handles_git_suffix() {
        assert_eq!(
            derive_repo_name("git@host:group/repo-name.git").as_deref(),
            Some("repo-name")
        );
    }
}
