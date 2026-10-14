//! Tests that need both capabilities' stores and the whole app, so none can live in
//! `identity-web`.

mod sign_in {
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use axum::Router;
    use chrono::Utc;
    use identity_core::cookie::{LOGIN, SESSION};
    use identity_core::records::Role;
    use identity_core::repository::SessionRepository;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::test_support::{
        body_string, cookie_set, get, location, open_test_db, post, state_over, test_app, test_state, urlencode,
        with_cookie, RecordingMailer, TestState,
    };

    const KNOWN: &str = "depositor@example.test";

    async fn a_user(state: &TestState, email: &str) -> Uuid {
        crate::test_support::a_user(state, email, "A Depositor", Role::Depositor, &[])
            .await
            .id
    }

    /// `POST /login` for `email`, carrying `binding` as the login cookie if given.
    async fn request_code(app: &Router, email: &str, binding: Option<&str>) -> Response {
        let form = format!("email={}", urlencode(email));
        let mut request = post("/login", &form);
        if let Some(binding) = binding {
            request = with_cookie(request, LOGIN, binding);
        }
        app.clone().oneshot(request).await.expect("the request should complete")
    }

    /// `POST /login/code` with `code`, bound by `binding`.
    async fn submit_code(app: &Router, code: &str, binding: &str) -> Response {
        let request = with_cookie(post("/login/code", &format!("code={code}")), LOGIN, binding);
        app.clone().oneshot(request).await.expect("the request should complete")
    }

    /// Drive a full sign-in and return the session cookie.
    async fn sign_in(app: &Router, mailer: &RecordingMailer, email: &str) -> String {
        let issued = request_code(app, email, None).await;
        let binding = cookie_set(&issued, LOGIN).expect("a binding must be handed out");
        let code = mailer.last_code().expect("a code must have been sent");
        let response = submit_code(app, &code, &binding).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "sign-in should redirect");
        cookie_set(&response, SESSION).expect("a session cookie must be set")
    }

    #[tokio::test]
    async fn test_a_code_signs_the_user_in_and_rotates_the_cookies() {
        let (state, mailer) = test_state("sign-in").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        let issued = request_code(&app, KNOWN, None).await;
        let binding = cookie_set(&issued, LOGIN).unwrap();
        let code = mailer.last_code().expect("a six-digit code should have been sent");
        assert_eq!(code.len(), 6);

        let signed_in = submit_code(&app, &code, &binding).await;
        assert_eq!(signed_in.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&signed_in).as_deref(), Some("/"));
        let session = cookie_set(&signed_in, SESSION).expect("a session cookie");
        assert!(!session.is_empty());
        assert_eq!(
            cookie_set(&signed_in, LOGIN).as_deref(),
            Some(""),
            "the binding has done its job and must not outlive the code it addressed"
        );

        // And the session actually works: `/projects` is behind the guard, so
        // reaching it at all is the proof, and the header names the account.
        let projects = app
            .clone()
            .oneshot(with_cookie(get("/projects"), SESSION, &session))
            .await
            .unwrap();
        assert_eq!(projects.status(), StatusCode::OK);
        assert!(body_string(projects).await.contains("A Depositor"));
    }

    #[tokio::test]
    async fn test_the_destination_survives_the_whole_sign_in_and_is_where_the_user_lands() {
        // The point of `next`: someone who follows a link into a project and is
        // sent to sign in ends up on the project, not on the root wondering
        // where they were going.
        let (state, mailer) = test_state("sign-in-next").await;
        let _ = crate::test_support::a_user(&state, KNOWN, "A Depositor", Role::Depositor, &["0801"]).await;
        let app = test_app(&state);

        // The guard names the destination.
        let guarded = app.clone().oneshot(get("/projects/0801")).await.unwrap();
        assert_eq!(location(&guarded).as_deref(), Some("/login?next=/projects/0801"));

        // The address form carries it on its action.
        let form = app.clone().oneshot(get("/login?next=/projects/0801")).await.unwrap();
        assert!(body_string(form).await.contains(r#"action="/login?next=/projects/0801""#));

        // Issuing carries it on to the code screen.
        let issued = app
            .clone()
            .oneshot(post("/login?next=/projects/0801", &format!("email={}", urlencode(KNOWN))))
            .await
            .unwrap();
        assert_eq!(location(&issued).as_deref(), Some("/login/code?next=/projects/0801"));
        let binding = cookie_set(&issued, LOGIN).expect("a binding");
        let code = mailer.last_code().expect("a code");

        // And spending the code lands on it.
        let signed_in = app
            .clone()
            .oneshot(with_cookie(
                post("/login/code?next=/projects/0801", &format!("code={code}")),
                LOGIN,
                &binding,
            ))
            .await
            .unwrap();
        assert_eq!(location(&signed_in).as_deref(), Some("/projects/0801"));
        let session = cookie_set(&signed_in, SESSION).expect("a session");

        // `/projects/{shortcode}` is a redirect into the form's first section, so
        // arriving is a 303 followed by a page — the point being that the
        // destination is reachable with the session just minted, not that it is
        // itself a page.
        let arrived = app
            .clone()
            .oneshot(with_cookie(get("/projects/0801"), SESSION, &session))
            .await
            .unwrap();
        assert_eq!(arrived.status(), StatusCode::SEE_OTHER);
        let section = location(&arrived).expect("the project redirects into its form");
        assert_eq!(section, "/projects/0801/sections/overview");

        let form = app
            .clone()
            .oneshot(with_cookie(get(&section), SESSION, &session))
            .await
            .unwrap();
        assert_eq!(form.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_a_session_past_its_absolute_expiry_is_refused_and_deleted() {
        let db = open_test_db("absolute-expiry").await;
        let mailer = RecordingMailer::new();
        let state = state_over(db.identity.clone(), db.editor.clone(), mailer.clone(), |auth| {
            auth.cooldown = Duration::ZERO
        });
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        // Age it past its absolute expiry, which is never extended.
        let expired = Utc::now() - chrono::TimeDelta::minutes(1);
        db.identity.set_session_expiry(&session, expired).await;

        let refused = app
            .clone()
            .oneshot(with_cookie(get("/projects"), SESSION, &session))
            .await
            .unwrap();
        assert_eq!(
            refused.status(),
            StatusCode::SEE_OTHER,
            "an expired session must not reach a guarded page"
        );
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects"));
        assert_eq!(
            SessionRepository::find(&*state.identity.store, &session).await.unwrap(),
            None,
            "and must not linger"
        );
    }

    #[tokio::test]
    async fn test_an_idle_session_is_refused_even_before_its_absolute_expiry() {
        let db = open_test_db("idle-expiry").await;
        let mailer = RecordingMailer::new();
        let state = state_over(db.identity.clone(), db.editor.clone(), mailer.clone(), |auth| {
            auth.cooldown = Duration::ZERO;
            auth.session_idle = Duration::from_secs(60);
        });
        a_user(&state, KNOWN).await;
        let app = test_app(&state);
        let session = sign_in(&app, &mailer, KNOWN).await;

        let stale = Utc::now() - chrono::TimeDelta::minutes(5);
        SessionRepository::touch(&*db.identity, &session, stale).await.unwrap();

        let refused = app
            .clone()
            .oneshot(with_cookie(get("/projects"), SESSION, &session))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&refused).as_deref(), Some("/login?next=/projects"));
        assert_eq!(SessionRepository::find(&*state.identity.store, &session).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_the_login_posts_are_refused_without_sec_fetch_site() {
        let (state, mailer) = test_state("csrf").await;
        a_user(&state, KNOWN).await;
        let app = test_app(&state);

        for (uri, form) in [
            ("/login", format!("email={}", urlencode(KNOWN))),
            ("/login/code", "code=123456".to_string()),
            ("/logout", String::new()),
        ] {
            let request = Request::builder()
                .method("POST")
                .uri(uri)
                .header("x-forwarded-for", "203.0.113.7")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "{uri} must be refused without the header"
            );
        }
        assert!(mailer.sent().is_empty());
    }
}

mod depositors {
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{TimeDelta, Utc};
    use editor_core::records::{DraftRecord, Submission, SubmissionState};
    use editor_core::repository::{DraftRepository, SubmissionRepository};
    use identity_core::cookie;
    use identity_core::records::{Role, User};
    use identity_core::repository::{SessionRepository, UserRepository};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::test_support::{
        a_session, a_user, body_string, get, location, open_test_db, post, state_over, test_app, test_state, urlencode,
        RecordingMailer, TestState,
    };

    const DEPOSITOR_EMAIL: &str = "a.depositor@example.test";

    fn as_session(request: Request<Body>, session: &str) -> Request<Body> {
        crate::test_support::with_cookie(request, cookie::SESSION, session)
    }

    /// An RDU account with a live session.
    async fn rdu(state: &TestState) -> (User, String) {
        let user = a_user(state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    /// A depositor account with a live session.
    async fn depositor(state: &TestState, email: &str, shortcodes: &[&str]) -> (User, String) {
        let user = a_user(state, email, "A Depositor", Role::Depositor, shortcodes).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    /// What the editor is told about `user` once they sign in: the access an assignment grants.
    async fn access_of(state: &TestState, user: &User) -> editor_ports::Principal {
        let session = a_session(state, user.id).await;
        state
            .authenticator
            .authenticate(&[&format!("{}={session}", cookie::SESSION)], std::time::SystemTime::now())
            .await
            .expect("a fresh session should authenticate")
    }

    fn form_body(name: &str, email: &str, shortcodes: &str) -> String {
        format!(
            "name={}&email={}&shortcodes={}",
            urlencode(name),
            urlencode(email),
            urlencode(shortcodes)
        )
    }

    #[tokio::test]
    async fn test_rdu_creates_a_depositor_with_a_name_address_and_projects() {
        let (state, _) = test_state("dep-create").await;
        let (_, session) = rdu(&state).await;
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(
                post("/depositors", &form_body("A Depositor", DEPOSITOR_EMAIL, "0801, 080C")),
                &session,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/depositors"));

        let created = UserRepository::find_by_email(&*state.identity.store, DEPOSITOR_EMAIL)
            .await
            .unwrap()
            .expect("the account should exist");
        assert_eq!(created.name, "A Depositor");
        assert_eq!(created.shortcodes, vec!["0801".to_string(), "080C".to_string()]);
        // The role is never taken from the form: RDU membership is
        // configuration's to decide.
        assert_eq!(created.role, Role::Depositor);
        let access = access_of(&state, &created).await;
        assert!(access.may_reach("0801"));
        assert!(!access.may_reach("0803"));
    }

    #[tokio::test]
    async fn test_an_edit_changes_the_three_fields_and_leaves_everything_else_alone() {
        // The record is rebuilt from the stored one, so the fields this surface
        // does not own survive. Without that, correcting a name would silently
        // clear the account's failure counter and its last-code stamp.
        let (state, _) = test_state("dep-update").await;
        let (_, session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801", "080C"]).await;
        let issued = Utc::now() - TimeDelta::hours(3);
        UserRepository::record_code_issued(&*state.identity.store, target.id, issued)
            .await
            .unwrap();
        UserRepository::record_failed_login(
            &*state.identity.store,
            target.id,
            Utc::now(),
            Utc::now() - TimeDelta::hours(1),
        )
        .await
        .unwrap();
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(
                post(
                    &format!("/depositors/{}/edit", target.id),
                    &form_body("A. Depositor", "renamed@example.test", "0812"),
                ),
                &session,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        let updated = UserRepository::find_by_id(&*state.identity.store, target.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "A. Depositor");
        assert_eq!(updated.email, "renamed@example.test");
        assert_eq!(updated.shortcodes, vec!["0812".to_string()]);
        assert_eq!(updated.role, Role::Depositor);
        assert_eq!(updated.failed_logins, 1, "the failure counter is not the form's to reset");
        assert!(updated.last_code_at.is_some(), "nor is the last-code stamp");
        assert_eq!(updated.created_at, target.created_at);

        // And the assignment change is what access now follows.
        let access = access_of(&state, &updated).await;
        assert!(!access.may_reach("0801"));
        assert!(access.may_reach("0812"));
    }

    #[tokio::test]
    async fn test_dropping_a_shortcode_leaves_the_projects_draft_alone() {
        // A draft is keyed by shortcode, not by user, so it belongs to the
        // project. Taking away an assignment takes away access and nothing else,
        // and RDU still sees the draft.
        let (state, _) = test_state("dep-update-draft").await;
        let (_, session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        DraftRepository::upsert(
            &*state.editor,
            &DraftRecord {
                shortcode: "0801".to_string(),
                payload: "{}".to_string(),
                updated_by: Some(target.id),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        app.clone()
            .oneshot(as_session(
                post(
                    &format!("/depositors/{}/edit", target.id),
                    &form_body("A Depositor", DEPOSITOR_EMAIL, ""),
                ),
                &session,
            ))
            .await
            .unwrap();

        let draft = DraftRepository::find(&*state.editor, "0801")
            .await
            .unwrap()
            .expect("the draft survives");
        assert_eq!(draft.updated_by, Some(target.id), "and still names its last editor");
    }

    #[tokio::test]
    async fn test_removal_deletes_the_account_and_every_session_belonging_to_it() {
        // The sessions half is what stops a removed depositor going on using the tab they
        // already had open.
        let db = open_test_db("dep-remove").await;
        let state = state_over(db.identity.clone(), db.editor.clone(), RecordingMailer::new(), |auth| {
            auth.cooldown = Duration::ZERO
        });
        let (_, rdu_session) = rdu(&state).await;
        let (target, their_session) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        let second_session = a_session(&state, target.id).await;
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(post(&format!("/depositors/{}/remove", target.id), ""), &rdu_session))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/depositors"));

        assert!(UserRepository::find_by_id(&*state.identity.store, target.id)
            .await
            .unwrap()
            .is_none());
        for session in [&their_session, &second_session] {
            assert_eq!(SessionRepository::find(&*state.identity.store, session).await.unwrap(), None);
        }
        // And the assignments went with it, so nothing is left pointing at a
        // row that is gone.
        assert_eq!(db.identity.count_rows("user_shortcodes").await, 0);

        // The session they were holding no longer opens anything.
        let after = app
            .clone()
            .oneshot(as_session(get("/projects/0801"), &their_session))
            .await
            .unwrap();
        assert_eq!(after.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&after).as_deref(), Some("/login?next=/projects/0801"));
    }

    #[tokio::test]
    async fn test_removal_keeps_the_work_and_makes_its_author_unknown() {
        // The editor's rows keep the author's id: the account is gone from
        // identity's store, so the id names nobody and the review queue's "last
        // editor" reads as unknown, while the project's work survives.
        let (state, _) = test_state("dep-remove-work").await;
        let (_, rdu_session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        DraftRepository::upsert(
            &*state.editor,
            &DraftRecord {
                shortcode: "0801".to_string(),
                payload: "{}".to_string(),
                updated_by: Some(target.id),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        SubmissionRepository::create(
            &*state.editor,
            &Submission {
                id: Uuid::new_v4(),
                shortcode: "0812".to_string(),
                payload: "{}".to_string(),
                state: SubmissionState::Submitted,
                submitted_by: Some(target.id),
                submitted_at: Utc::now(),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
                review_state: None,
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        app.clone()
            .oneshot(as_session(post(&format!("/depositors/{}/remove", target.id), ""), &rdu_session))
            .await
            .unwrap();

        let draft = DraftRepository::find(&*state.editor, "0801")
            .await
            .unwrap()
            .expect("the draft survives");
        assert_eq!(draft.updated_by, Some(target.id));
        let submissions = SubmissionRepository::list(&*state.editor).await.unwrap();
        assert_eq!(submissions.len(), 1, "the submission survives and stays pending");
        assert_eq!(submissions[0].submitted_by, Some(target.id));
        assert_eq!(
            UserRepository::find_by_id(&*state.identity.store, target.id).await.unwrap(),
            None,
            "the id names no account any more"
        );
    }

    #[tokio::test]
    async fn test_the_confirmation_names_the_address_and_the_work_that_becomes_unreturnable() {
        // Both are irreversible and neither is obvious: the address is RDU's
        // only channel to this person, and a pending submission with no author
        // can be approved or rejected but never returned.
        let (state, _) = test_state("dep-remove-confirm").await;
        let (_, rdu_session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        SubmissionRepository::create(
            &*state.editor,
            &Submission {
                id: Uuid::new_v4(),
                shortcode: "0801".to_string(),
                payload: "{}".to_string(),
                state: SubmissionState::Submitted,
                submitted_by: Some(target.id),
                submitted_at: Utc::now(),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
                review_state: None,
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(get(&format!("/depositors/{}/remove", target.id)), &rdu_session))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains(DEPOSITOR_EMAIL), "{body}");
        assert!(body.contains("0801"), "{body}");
        assert!(body.contains("no longer be returned"), "{body}");

        assert!(
            UserRepository::find_by_id(&*state.identity.store, target.id)
                .await
                .unwrap()
                .is_some(),
            "the confirmation must change nothing"
        );
    }

    #[tokio::test]
    async fn test_every_write_shares_its_url_with_a_get_that_renders_a_page() {
        // Two invariants.
        //
        // A state-changing `GET` is the one thing the `Sec-Fetch-Site` control
        // cannot cover, because a navigation from anywhere is a `GET` — so no
        // `GET` here may write.
        //
        // And every write must post to a URL that *answers* `GET`, because a
        // rejected submission re-renders at the path it posted to. A write
        // posting to a path with no `GET` strands it on a bare 405 — no body, no
        // shell, no way back.
        let (state, _) = test_state("dep-method").await;
        let (_, session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        let app = test_app(&state);

        for uri in [
            "/depositors".to_string(),
            format!("/depositors/{}/edit", target.id),
            format!("/depositors/{}/remove", target.id),
        ] {
            let response = app.clone().oneshot(as_session(get(&uri), &session)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "every write URL must answer GET: {uri}");
        }

        // The old URL is gone rather than merely unused, so nothing can post to
        // a path with no `GET` again without adding one back. The two statuses
        // differ because an unmatched request falls through to the `ServeDir`
        // fallback, whose not-found service is `GET`-only: a `GET` reaches the
        // 404 page, a `POST` gets 405 from the fallback itself. Neither is the
        // update handler, which is the point.
        let gone = app
            .clone()
            .oneshot(as_session(get(&format!("/depositors/{}", target.id)), &session))
            .await
            .unwrap();
        assert_eq!(gone.status(), StatusCode::NOT_FOUND);
        let gone = app
            .clone()
            .oneshot(as_session(post(&format!("/depositors/{}", target.id), ""), &session))
            .await
            .unwrap();
        assert_eq!(gone.status(), StatusCode::METHOD_NOT_ALLOWED);

        let after = UserRepository::find_by_id(&*state.identity.store, target.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.name, "A Depositor", "no GET on this surface may change anything");
        assert_eq!(after.shortcodes, vec!["0801".to_string()]);
    }
}

mod sections {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use identity_core::records::Role;
    use tower::ServiceExt;

    use crate::test_support::{a_session, a_user, test_app, test_state};

    const OVERVIEW: &str = "/projects/0801d/sections/overview";

    #[tokio::test]
    async fn a_save_with_no_sec_fetch_site_never_reaches_the_handler() {
        // The CSRF control is the outermost layer, and every write in this
        // service depends on it. Asserted here because a new write route is
        // exactly where the assumption would go untested.
        let (state, _) = test_state("section-csrf").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let mut request = Request::builder()
            .method("POST")
            .uri(OVERVIEW)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("name=Cross+Site"))
            .expect("the request should build");
        request.headers_mut().append(
            axum::http::header::COOKIE,
            format!("{}={session}", identity_core::cookie::SESSION)
                .parse()
                .expect("a cookie"),
        );
        let response = app.clone().oneshot(request).await.expect("completes");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
