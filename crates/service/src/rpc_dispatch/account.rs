use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

use crate::{account_list, auth_account, auth_login, auth_tokens};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "account/list" => super::value_or_error(account_list::read_accounts()),
        "account/delete" => super::ok_or_error(crate::account::management::delete_account(
            super::str_param(req, "accountId").unwrap_or(""),
        )),
        "account/deleteMany" => super::value_or_error(crate::account::management::delete_accounts(
            string_array_param(req, "accountIds"),
        )),
        "account/deleteUnavailableFree" => {
            super::value_or_error(crate::account::management::delete_unavailable_free_accounts())
        }
        "account/deleteByStatuses" => {
            super::value_or_error(crate::account::management::delete_accounts_by_statuses(
                string_array_param(req, "statuses"),
            ))
        }
        "account/update" => {
            let has_note = has_param(req, "note");
            let has_tags = has_param(req, "tags");
            super::ok_or_error(crate::account::management::update_account(
                super::str_param(req, "accountId").unwrap_or(""),
                super::i64_param(req, "sort"),
                super::str_param(req, "status"),
                super::str_param(req, "label"),
                super::str_param(req, "note"),
                super::str_param(req, "tags"),
                has_note,
                has_tags,
            ))
        }
        "account/login/start" => {
            let login_type = super::str_param(req, "type").unwrap_or("chatgpt");
            if login_type.eq_ignore_ascii_case("chatgptAuthTokens") {
                super::value_or_error(auth_account::login_with_chatgpt_auth_tokens(
                    auth_account::ChatgptAuthTokensLoginInput {
                        access_token: first_string_param(req, &["accessToken", "access_token"])
                            .unwrap_or_default(),
                        refresh_token: first_string_param(req, &["refreshToken", "refresh_token"]),
                        id_token: first_string_param(req, &["idToken", "id_token"]),
                        chatgpt_account_id: first_string_param(
                            req,
                            &["chatgptAccountId", "chatgpt_account_id", "accountId"],
                        ),
                        workspace_id: first_string_param(req, &["workspaceId", "workspace_id"]),
                        chatgpt_plan_type: first_string_param(
                            req,
                            &["chatgptPlanType", "chatgpt_plan_type", "planType"],
                        ),
                    },
                ))
            } else {
                let open_browser = super::bool_param(req, "openBrowser").unwrap_or(false);
                super::value_or_error(auth_login::login_start(
                    login_type,
                    open_browser,
                    None,
                    None,
                    None,
                    None,
                ))
            }
        }
        "account/login/status" => {
            let login_id = super::str_param(req, "loginId").unwrap_or("");
            super::as_json(auth_login::login_status(login_id))
        }
        "account/login/complete" => {
            let state = super::str_param(req, "state").unwrap_or("");
            let code = super::str_param(req, "code").unwrap_or("");
            let redirect_uri = super::str_param(req, "redirectUri");
            if state.is_empty() || code.is_empty() {
                serde_json::json!({"ok": false, "error": "missing code/state"})
            } else {
                super::ok_or_error(auth_tokens::complete_login_with_redirect(
                    state,
                    code,
                    redirect_uri,
                ))
            }
        }
        "account/chatgptAuthTokens/refresh" => {
            let account_id = first_str_param(req, &["accountId", "account_id"])
                .or_else(|| first_str_param(req, &["previousAccountId", "previous_account_id"]));
            super::value_or_error(auth_account::refresh_current_chatgpt_auth_tokens(
                account_id,
            ))
        }
        "account/chatgptAuthTokens/refreshAll" => {
            super::value_or_error(auth_account::refresh_all_chatgpt_auth_tokens())
        }
        "account/read" => super::value_or_error(auth_account::read_current_account(
            first_bool_param(req, &["refreshToken", "refresh_token"]).unwrap_or(false),
        )),
        "account/logout" => super::value_or_error(auth_account::logout_current_account()),
        _ => return None,
    };
    Some(super::response(req, result))
}

fn string_array_param(req: &JsonRpcRequest, key: &str) -> Vec<String> {
    req.params
        .as_ref()
        .and_then(|params| params.get(key))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn has_param(req: &JsonRpcRequest, key: &str) -> bool {
    req.params
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .is_some_and(|params| params.contains_key(key))
}

fn first_str_param<'a>(req: &'a JsonRpcRequest, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| super::str_param(req, key))
}

fn first_string_param(req: &JsonRpcRequest, keys: &[&str]) -> Option<String> {
    first_str_param(req, keys).map(ToString::to_string)
}

fn first_bool_param(req: &JsonRpcRequest, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| super::bool_param(req, key))
}
