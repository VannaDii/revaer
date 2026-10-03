
const {execFileSync,spawn}=require("node:child_process");
const {createMediaServiceRelay}=require("./media-service-relay.cjs");
const {chromium,expect}=require("../node_modules/@playwright/test");
const app=process.env.APP_CONTAINER;
if(!app)throw Error("APP_CONTAINER is required from the owned fixture");
let browser,ui,page;
const browserErrors=[];

const fixtureRelay=createMediaServiceRelay(app);
const relay=fixtureRelay.server;
async function call(method,path,body,headers={}){
 const response=await fetch("http://localhost:7070"+path,{method,headers:{...headers,...(body===undefined?{}:{"Content-Type":"application/json"})},body:body===undefined?undefined:JSON.stringify(body)});
 const raw=await response.text();
 let data;try{data=JSON.parse(raw)}catch{data=raw}
 return {status:response.status,data,headers:response.headers};
}
function required(result,status,stage){if(result.status!==status)throw Error(stage+" "+JSON.stringify(result));return result.data}
async function waitReady(url){
 let lastFailure="HTTP not ready";
 for(let i=0;i<120;i++){
  try{const result=await fetch(url);if(result.ok)return}catch(error){lastFailure=error.message}
  if(ui&&ui.exitCode!==null)throw Error("UI compiler exited");
  await new Promise(resolve=>setTimeout(resolve,500));
 }
 throw Error("fixture HTTP readiness timed out: "+lastFailure);
}
(async()=>{
 await new Promise((resolve,reject)=>{relay.once("error",reject);relay.listen(7070,"127.0.0.1",resolve)});
 const start=required(await call("POST","/admin/setup/start",{}),200,"setup start");
 const snapshot=required(await call("GET","/.well-known/revaer.json"),200,"snapshot");
 const setup=required(await call("POST","/admin/setup/complete",{app_profile:{...snapshot.app_profile,auth_mode:"api_key"},fs_policy:{...snapshot.fs_policy,allow_paths:["/proof"]}},{"x-revaer-setup-token":start.token}),200,"setup");
 if(!setup.api_key)throw Error("fixture setup did not return key");
 const headers={"x-revaer-api-key":setup.api_key};
 required(await call("POST","/v1/media/targets",{target_key:"ui-target",version:1,display_name:"UI fixture target",container_format:"matroska",streams:[{stream_key:"main-video",stream_kind:"video",optional:false,sort_order:0,codec:"h264",default_disposition:true,forced_disposition:false}]},headers),201,"fixture target");
 execFileSync("just",["sync-assets"],{stdio:"inherit"});
 ui=spawn("trunk",["serve","--no-autoreload=true","--dist","dist-serve","--port","8080"],{cwd:"crates/revaer-ui",env:{...process.env,REVAER_UI_API_BASE_URL:"http://localhost:7070",NO_COLOR:"true"},stdio:"inherit"});
 await waitReady("http://localhost:8080");
 browser=await chromium.launch();
 const context=await browser.newContext({viewport:{width:1280,height:900}});
 await context.addInitScript(key=>{
  localStorage.setItem("revaer.auth.mode",JSON.stringify("api_key"));
  localStorage.setItem("revaer.api_key",JSON.stringify(key));
  localStorage.setItem("revaer.api_key_expires_at",JSON.stringify(Date.now()+86400000));
 },setup.api_key);
 page=await context.newPage();
 page.on("pageerror",error=>browserErrors.push(error.message));
 await page.goto("http://localhost:8080/media");

 const policyForm=page.getByTestId("media-policy-form");
 await policyForm.getByPlaceholder("policy_catalog_key").fill("ui-execution");
 await policyForm.getByPlaceholder("policy_version").fill("1");
 await policyForm.getByPlaceholder("policy_display_name").fill("UI execution");
 await expect(policyForm.getByLabel("Policy dry-run",{exact:true})).toBeChecked();
 await policyForm.getByLabel("Policy dry-run",{exact:true}).uncheck();
 await policyForm.getByLabel("Replacement mode",{exact:true}).selectOption("atomic_replace");
 const policyResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/policies");
 await policyForm.getByRole("button",{name:"Save policy",exact:true}).click();
 const policySaved=await policyResponse;
 if(policySaved.status()!==201)throw Error("UI output policy save "+policySaved.status()+" "+await policySaved.text());
 const configuredPolicy=await policySaved.json();
 const expectedOutput={dry_run:false,replacement_mode:"atomic_replace",quarantine_enabled:true,preserve_permissions:true,preserve_ownership:true};
 for(const [key,value] of Object.entries(expectedOutput))if(configuredPolicy.output[key]!==value)throw Error("UI output policy setting mismatch "+key);
 const policyRead=required(await call("GET","/v1/media/policies",undefined,headers),200,"output policy readback");
 const persistedPolicy=policyRead.policies.find(row=>row.policy_key==="ui-execution"&&row.version===1);
 if(!persistedPolicy||JSON.stringify(persistedPolicy.output)!==JSON.stringify(configuredPolicy.output))throw Error("output policy readback mismatch");
 console.log("REAL_BROWSER_OUTPUT_POLICY_CREATE_AND_COMPLETE_READBACK");
 const editor=page.getByTestId("media-profile-root-editor");
 await expect(editor).toBeVisible({timeout:30000});
 for(const [label,value] of [["Profile key","ui-milestone"],["Display name","UI milestone"],["Desired target key","ui-target"],["Desired target version","1"],["Policy key","ui-execution"],["Policy version","1"]])await editor.getByLabel(label,{exact:true}).fill(value);
 await editor.getByLabel("Enabled",{exact:true}).check();
 await expect(editor.getByLabel("Dry-run only",{exact:true})).toBeChecked();
 await editor.getByLabel("Dry-run only",{exact:true}).uncheck();
 for(const [label,key] of [["Output root","source"],["Workspace root","workspace"],["Quarantine root","quarantine"]])await editor.getByRole("combobox",{name:label,exact:true}).selectOption(key);
 const profileResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/profiles");
 await editor.getByRole("button",{name:"Create profile",exact:true}).click();
 const savedResponse=await profileResponse;
 if(savedResponse.status()!==201)throw Error("UI profile save "+savedResponse.status()+" "+await savedResponse.text());
 const profile=await savedResponse.json();
 const profilePath="/v1/media/profiles/"+profile.media_profile_public_id;
 const savedEtag=savedResponse.headers()["etag"];
 if(savedEtag!==`"media-profile:${profile.media_profile_public_id}:v1"`)throw Error("profile ETag is not the exact saved version");
 if(savedResponse.headers()["location"]!==profilePath)throw Error("profile Location is not its saved resource");
 const persistedProfile=required(await call("GET",profilePath,undefined,headers),200,"profile persistence");
 expect(persistedProfile).toEqual(profile);
 const completeProfile={};
 for(const key of ["profile_key","display_name","description","enabled","dry_run_only",
  "desired_target_key","desired_target_version","policy_key","policy_version",
  "output_root_key","workspace_root_key","backup_root_key","quarantine_root_key"]){
  if(Object.hasOwn(profile,key))completeProfile[key]=profile[key];
 }
 const sourceHashBeforeConflict=execFileSync("docker",["exec",app,"sha256sum","/proof/source/Movies/original.mkv"],{encoding:"utf8"}).split(" ")[0];
 const duplicate=await call("POST","/v1/media/profiles",{
  ...completeProfile,description:"Duplicate must not alter the saved version",
 },{...headers,"If-None-Match":"*"});
 required(duplicate,409,"duplicate profile rejection");
 expect(duplicate.data.context).toContainEqual({name:"error_code",value:"media_profile_key_conflict"});
 expect(required(await call("GET",profilePath,undefined,headers),200,"profile after conflict")).toEqual(persistedProfile);
 for(const field of ["source_root","output_root","schedule_enabled","watcher_enabled"]){
  const invalid=await call("POST","/v1/media/profiles",{
   ...completeProfile,profile_key:"invalid-"+field.replaceAll("_","-"),[field]:field.endsWith("root")?"/private/not-authority":true,
  },{...headers,"If-None-Match":"*"});
  required(invalid,400,"physical/automation profile field rejection");
  expect(invalid.data.context).toContainEqual({name:"error_code",value:"media_configuration_invalid"});
 }
 const afterConflict=required(await call("GET","/v1/media/profiles?limit=50",undefined,headers),200,"profile collection after rejected writes");
 if(afterConflict.profiles.some(row=>row.profile_key.startsWith("invalid-")))throw Error("invalid profile write persisted a parent");
 const sourceHashAfterConflict=execFileSync("docker",["exec",app,"sha256sum","/proof/source/Movies/original.mkv"],{encoding:"utf8"}).split(" ")[0];
 if(sourceHashBeforeConflict!==sourceHashAfterConflict)throw Error("profile authoring changed source bytes");
 console.log("REAL_PROFILE_PERSISTENCE_CONFLICT_AND_PATH_FREE_CONTRACT");
 const replacementBody={...completeProfile,description:"Complete immutable replacement from the current version"};
 await expect(editor.getByRole("status").filter({hasText:"Profile saved as version 1."})).toHaveText("Profile saved as version 1.");
 page.once("dialog",dialog=>dialog.accept());
 await page.getByRole("button",{name:"Edit profile ui-milestone",exact:true}).click();
 await expect(editor.getByLabel("Profile key",{exact:true})).toBeDisabled();
 await expect(editor.getByLabel("Display name",{exact:true})).toHaveValue(completeProfile.display_name);
 await editor.getByLabel("Description",{exact:true}).fill(replacementBody.description);
 const replacementResponse=page.waitForResponse(r=>r.request().method()==="PUT"&&new URL(r.url()).pathname===profilePath);
 await editor.getByRole("button",{name:"Save profile",exact:true}).click();
 const replacementResult=await replacementResponse;
 expect(replacementResult.request().headers()["if-match"]).toBe(savedEtag);
 const replaced={status:replacementResult.status(),data:await replacementResult.json(),headers:new Headers(replacementResult.headers())};
 const versionTwo=required(replaced,200,"complete profile replacement");
 expect(versionTwo.latest_version).toBe(2);
 expect(versionTwo.active_version).toBe(2);
 expect(versionTwo.description).toBe(replacementBody.description);
 const versionTwoEtag=`"media-profile:${profile.media_profile_public_id}:v2"`;
 expect(replaced.headers.get("etag")).toBe(versionTwoEtag);
 expect(required(await call("GET",profilePath,undefined,headers),200,"replacement persistence")).toEqual(versionTwo);
 const stale=await call("PUT",profilePath,{...replacementBody,description:"Stale edit must not persist"},{...headers,"If-Match":savedEtag});
 required(stale,412,"stale profile replacement");
 expect(stale.data.context).toContainEqual({name:"error_code",value:"media_configuration_version_conflict"});
 required(await call("PUT",profilePath,replacementBody,headers),428,"missing replacement precondition");
 required(await call("PUT",profilePath,{...replacementBody,profile_key:"renamed-profile"},{...headers,"If-Match":versionTwoEtag}),400,"immutable profile key");
 const patch=await call("PATCH",profilePath,{dry_run_only:true},headers);
 required(patch,405,"retired mutable profile patch");
 expect(patch.headers.get("allow")).toBe("GET, HEAD, PUT");
 expect(required(await call("GET",profilePath,undefined,headers),200,"profile after rejected replacements")).toEqual(versionTwo);
 console.log("REAL_PROFILE_IMMUTABLE_REPLACEMENT_AND_STALE_EDIT_REJECTION");
 await expect(editor.getByRole("status").filter({hasText:"Profile saved as version 2."})).toHaveText("Profile saved as version 2.");
 page.once("dialog",dialog=>dialog.accept());
 await page.getByRole("button",{name:"Edit profile ui-milestone",exact:true}).click();
 await expect(editor.getByLabel("Description",{exact:true})).toHaveValue(replacementBody.description);
 await editor.getByLabel("Description",{exact:true}).fill("Operator draft preserved after a stale save");
 const foreign=required(await call("PUT",profilePath,{...replacementBody,description:"Another operator saved version 3"},{...headers,"If-Match":versionTwoEtag}),200,"intervening profile replacement");
 expect(foreign.latest_version).toBe(3);
 const staleUiResponse=page.waitForResponse(r=>r.request().method()==="PUT"&&new URL(r.url()).pathname===profilePath);
 await editor.getByRole("button",{name:"Save profile",exact:true}).click();
 const staleUiResult=await staleUiResponse;
 expect(staleUiResult.status()).toBe(412);
 expect(staleUiResult.request().headers()["if-match"]).toBe(versionTwoEtag);
 await expect(editor.getByRole("alert")).toContainText("preserved draft");
 await expect(editor.getByLabel("Description",{exact:true})).toHaveValue("Operator draft preserved after a stale save");
 expect(required(await call("GET",profilePath,undefined,headers),200,"profile after stale UI save")).toEqual(foreign);
 await page.screenshot({path:"target/profile-edit-desktop.png"});
 await page.setViewportSize({width:375,height:900});
 if(await page.getByLabel("Toggle layout sidebar",{exact:true}).isChecked()){
  const backdrop=page.locator("#layout-sidebar-backdrop");
  const bounds=await backdrop.boundingBox();
  if(!bounds)throw Error("mobile sidebar backdrop is unavailable");
  await backdrop.click({position:{x:bounds.width-10,y:bounds.height/2}});
  await expect(page.getByLabel("Toggle layout sidebar",{exact:true})).not.toBeChecked();
 }
 await expect.poll(()=>page.locator("#layout-sidebar").evaluate(element=>element.getBoundingClientRect().right)).toBeLessThanOrEqual(1);
 await expect(editor.getByRole("button",{name:"Save profile",exact:true})).toBeVisible();
 if(await editor.evaluate(element=>element.scrollWidth>element.clientWidth+1))throw Error("profile editor overflows mobile width");
 const mobileSave=page.waitForResponse(r=>r.request().method()==="PUT"&&new URL(r.url()).pathname===profilePath);
 await editor.getByRole("button",{name:"Save profile",exact:true}).click();
 expect((await mobileSave).status()).toBe(412);
 await expect(editor.getByLabel("Description",{exact:true})).toHaveValue("Operator draft preserved after a stale save");
 await page.screenshot({path:"target/profile-edit-mobile.png"});
 await page.setViewportSize({width:1280,height:900});
 console.log("REAL_BROWSER_PROFILE_EDIT_AND_STALE_DRAFT_PRESERVATION");
 await page.reload();
 const association=page.getByTestId("media-association-editor");
 await expect(association).toBeVisible({timeout:30000});
 await association.getByLabel("Association key",{exact:true}).fill("ui-manual");
 await association.getByLabel("Active profile version").selectOption(profile.media_profile_public_id);
 await association.getByRole("combobox",{name:"Source root",exact:true}).selectOption("source");
 await association.getByRole("radio",{name:"Relative prefix",exact:true}).check();
 await association.getByRole("textbox",{name:"Relative prefix",exact:true}).fill("Movies");
 await association.getByLabel("Manual",{exact:true}).check();
 const associationResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/discovery-associations");
 await association.getByRole("button",{name:"Create association",exact:true}).click();
 const boundResponse=await associationResponse;
 if(boundResponse.status()!==201)throw Error("UI association save "+boundResponse.status()+" "+await boundResponse.text());
 const bound=await boundResponse.json();
 await expect(page.getByTestId("media-manual-discovery")).toBeVisible();
 await page.reload();
 await expect(editor).toBeVisible({timeout:30000});
 for(const [label,value] of [["Profile key","ui-dry-plan"],["Display name","Dry-run plan"],["Desired target key","ui-target"],["Desired target version","1"],["Policy key","ui-execution"],["Policy version","1"]])await editor.getByLabel(label,{exact:true}).fill(value);
 await editor.getByLabel("Enabled",{exact:true}).check();
 await expect(editor.getByLabel("Dry-run only",{exact:true})).toBeChecked();
 for(const [label,key] of [["Output root","source"],["Workspace root","workspace"],["Quarantine root","quarantine"]])await editor.getByRole("combobox",{name:label,exact:true}).selectOption(key);
 const dryProfileResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/profiles");
 await editor.getByRole("button",{name:"Create profile",exact:true}).click();
 const drySavedResponse=await dryProfileResponse;
 expect(drySavedResponse.status()).toBe(201);
 const dryProfile=await drySavedResponse.json();
 expect(dryProfile.dry_run_only).toBe(true);
 await association.getByLabel("Association key",{exact:true}).fill("ui-dry-manual");
 await association.getByLabel("Active profile version").selectOption(dryProfile.media_profile_public_id);
 await association.getByRole("combobox",{name:"Source root",exact:true}).selectOption("source");
 await association.getByRole("radio",{name:"Relative prefix",exact:true}).check();
 await association.getByRole("textbox",{name:"Relative prefix",exact:true}).fill("DryRun");
 await association.getByLabel("Manual",{exact:true}).check();
 const dryAssociationResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/discovery-associations");
 await association.getByRole("button",{name:"Create association",exact:true}).click();
 const dryBoundResponse=await dryAssociationResponse;
 expect(dryBoundResponse.status()).toBe(201);
 const dryBound=await dryBoundResponse.json();
 console.log("REAL_BROWSER_AUTHENTICATED_PROFILE_AND_ASSOCIATION_SAVE");
 const schedulePath="/v1/media/discovery-associations/"+dryBound.media_discovery_association_public_id+"/schedule";
 const scheduleSelector=page.getByTestId("media-schedule-selector");
 await scheduleSelector.getByRole("combobox",{name:"Schedule association",exact:true}).selectOption(dryBound.media_discovery_association_public_id);
 const schedule=page.getByTestId("media-schedule-authoring");
 await expect(schedule.getByRole("button",{name:"Save cadence",exact:true})).toBeEnabled();
 await expect(schedule.getByLabel("Schedule interval",{exact:true})).toHaveValue("");
 await expect(schedule.getByLabel("Interval unit",{exact:true})).toHaveValue("");
 await schedule.getByLabel("Schedule interval",{exact:true}).fill("2");
 await schedule.getByLabel("Interval unit",{exact:true}).selectOption("hours");
 const cadenceResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname===schedulePath);
 await schedule.getByRole("button",{name:"Save cadence",exact:true}).click();
 const cadenceSavedResponse=await cadenceResponse;
 expect(cadenceSavedResponse.status()).toBe(201);
 let savedSchedule=await cadenceSavedResponse.json();
 expect(savedSchedule).toMatchObject({interval_quantity:2,interval_unit:"hours"});
 await expect(schedule).toContainText("Saved cadence: 2 hours");
 console.log("REAL_BROWSER_EXPLICIT_SCHEDULE_CADENCE_SAVE");
 await schedule.getByRole("button",{name:"Edit cadence",exact:true}).click();
 await schedule.getByLabel("Schedule interval",{exact:true}).fill("5");
 await schedule.getByLabel("Interval unit",{exact:true}).selectOption("minutes");
 const editCadenceResponse=page.waitForResponse(r=>r.request().method()==="PUT"&&new URL(r.url()).pathname===schedulePath);
 await schedule.getByRole("button",{name:"Save cadence",exact:true}).click();
 const editedCadenceResponse=await editCadenceResponse;
 expect(editedCadenceResponse.status()).toBe(200);
 expect(editedCadenceResponse.request().headers()["if-match"]).toBe(cadenceSavedResponse.headers()["etag"]);
 const initialAnchor=savedSchedule.anchor_due_at;
 savedSchedule=await editedCadenceResponse.json();
 expect(savedSchedule).toMatchObject({interval_quantity:5,interval_unit:"minutes",anchor_due_at:initialAnchor});
 await expect(schedule).toContainText("Saved cadence: 5 minutes");
 await schedule.getByRole("button",{name:"Edit cadence",exact:true}).click();
 await schedule.getByLabel("Schedule interval",{exact:true}).fill("6");
 const interveningCadence=await call("PUT",schedulePath,
  {association_version:dryBound.latest_version,interval_quantity:7,interval_unit:"hours"},
  {...headers,"If-Match":editedCadenceResponse.headers()["etag"]});
 savedSchedule=required(interveningCadence,200,"intervening cadence replacement");
 const staleCadenceResponse=page.waitForResponse(r=>r.request().method()==="PUT"&&new URL(r.url()).pathname===schedulePath);
 await schedule.getByRole("button",{name:"Save cadence",exact:true}).click();
 expect((await staleCadenceResponse).status()).toBe(412);
 await expect(schedule.getByRole("alert")).toContainText("Draft preserved");
 await expect(schedule.getByLabel("Schedule interval",{exact:true})).toHaveValue("6");
 await expect(schedule.getByLabel("Interval unit",{exact:true})).toHaveValue("minutes");
 await expect(schedule.getByRole("button",{name:"Save cadence",exact:true})).toBeDisabled();
 await schedule.getByRole("button",{name:"Reload cadence",exact:true}).click();
 await expect(schedule).toContainText("Saved cadence: 7 hours");
 await expect(schedule.getByLabel("Interval unit",{exact:true})).toHaveValue("hours");
 console.log("REAL_BROWSER_SCHEDULE_EDIT_AND_STALE_DRAFT_PRESERVATION");
 await scheduleSelector.screenshot({path:"target/schedule-cadence-desktop.png"});
 await page.setViewportSize({width:375,height:900});
 if(await page.getByLabel("Toggle layout sidebar",{exact:true}).isChecked()){
  const backdrop=page.locator("#layout-sidebar-backdrop");
  const bounds=await backdrop.boundingBox();
  if(!bounds)throw Error("mobile sidebar backdrop is unavailable");
  await backdrop.click({position:{x:bounds.width-10,y:bounds.height/2}});
 }
 await expect.poll(()=>page.locator("#layout-sidebar").evaluate(element=>element.getBoundingClientRect().right)).toBeLessThanOrEqual(1);
 await expect(scheduleSelector).toBeVisible();
 const cadenceFits=await scheduleSelector.evaluate(element=>{
  const controls=[...element.querySelectorAll("input,select,button")];
  return element.getBoundingClientRect().right<=window.innerWidth
   && controls.every(control=>control.getBoundingClientRect().right<=window.innerWidth);
 });
 expect(cadenceFits).toBe(true);
 await scheduleSelector.screenshot({path:"target/schedule-cadence-mobile.png"});
 await page.setViewportSize({width:1280,height:900});
 await scheduleSelector.getByRole("combobox",{name:"Schedule association",exact:true}).selectOption(bound.media_discovery_association_public_id);
 await expect(schedule.getByRole("button",{name:"Save cadence",exact:true})).toBeEnabled();
 await schedule.getByLabel("Schedule interval",{exact:true}).fill("3");
 await schedule.getByLabel("Interval unit",{exact:true}).selectOption("minutes");
 const conflictingPath="/v1/media/discovery-associations/"+bound.media_discovery_association_public_id+"/schedule";
 const otherCadence=required(await call("POST",conflictingPath,
  {association_version:bound.latest_version,interval_quantity:4,interval_unit:"hours"},
  {...headers,"If-None-Match":"*"}),201,"concurrent cadence save");
 const conflictingResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname===conflictingPath);
 await schedule.getByRole("button",{name:"Save cadence",exact:true}).click();
 expect((await conflictingResponse).status()).toBe(409);
 await expect(schedule.getByRole("alert")).toContainText("Draft preserved");
 await expect(schedule.getByLabel("Schedule interval",{exact:true})).toHaveValue("3");
 await expect(schedule.getByLabel("Interval unit",{exact:true})).toHaveValue("minutes");
 await expect(schedule.getByRole("button",{name:"Save cadence",exact:true})).toBeDisabled();
 await schedule.getByRole("button",{name:"Reload cadence",exact:true}).click();
 await expect(schedule).toContainText("Saved cadence: 4 hours");
 await expect(schedule.getByLabel("Interval unit",{exact:true})).toHaveValue("hours");
 console.log("REAL_BROWSER_SCHEDULE_CONFLICT_PRESERVES_DRAFT_AND_RELOADS_PERSISTED_CADENCE");
 // Restart the real service without replacing its attested mount namespace.
 execFileSync("docker",["exec",app,"touch","/proof/restart"],{stdio:"inherit"});
 let restarted=false;
 for(let i=0;i<80;i++){
  const pid=execFileSync("docker",["exec",app,"bash","-c","if [[ -f /proof/restarted-pid ]]; then cat /proof/restarted-pid; fi"],{encoding:"utf8"}).trim();
  if(/^[1-9][0-9]*$/.test(pid)){restarted=true;break;}
  await new Promise(resolve=>setTimeout(resolve,250));
 }
 if(!restarted)throw Error("real service process did not restart");
 await waitReady("http://localhost:7070/health");
 let bindingReady=false;
 for(let i=0;i<80;i++){
  const collection=required(await call("GET","/v1/media/discovery-associations?limit=50",undefined,headers),200,"restart association readiness");
  if([bound,dryBound].every(expected=>collection.associations.some(row=>row.media_discovery_association_public_id===expected.media_discovery_association_public_id&&row.binding_ready))){
   bindingReady=true;break;
  }
  await new Promise(resolve=>setTimeout(resolve,250));
 }
 if(!bindingReady)throw Error("association did not become ready after restart");
 expect(required(await call("GET",schedulePath,undefined,headers),200,"restarted schedule configuration")).toEqual(savedSchedule);
 expect(required(await call("GET",conflictingPath,undefined,headers),200,"restarted concurrent schedule configuration")).toEqual(otherCadence);
 console.log("REAL_SERVICE_RESTART_RETAINS_EXPLICIT_SCHEDULE_WITHOUT_ACTIVATION");
 await page.reload();
 const selector=page.getByTestId("media-association-list").getByRole("combobox",{name:"Manual association"});
 await expect(selector.locator("option")).toHaveCount(3,{timeout:30000});
 await selector.selectOption(dryBound.media_discovery_association_public_id);
 const manual=page.getByTestId("media-manual-discovery");
 const sourceHash=()=>execFileSync("docker",["exec",app,"sha256sum","/proof/source/DryRun/original.mkv"],{encoding:"utf8"}).split(" ")[0];
 const beforeDryPlan=sourceHash();
 expect(required(await call("GET","/v1/media/profiles/"+dryProfile.media_profile_public_id,undefined,headers),200,"restarted dry-run profile")).toEqual(dryProfile);
 await manual.getByLabel("Relative candidates").fill("DryRun/original.mkv");
 await manual.getByRole("button",{name:"Preview candidates",exact:true}).click();
 await expect(manual).toContainText("DryRun/original.mkv | Accepted | Dry-run");
 await manual.getByLabel("Queue jobs using the active profile").check();
 const dryAdmissionResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/discovery/runs");
 await manual.getByRole("button",{name:"Queue jobs",exact:true}).click();
 const dryAdmittedResponse=await dryAdmissionResponse;
 expect(dryAdmittedResponse.status()).toBe(201);
 const dryAdmitted=await dryAdmittedResponse.json();
 expect(dryAdmitted.queued_jobs).toHaveLength(1);
 expect(dryAdmitted.skipped).toEqual([]);
 const dryJobId=dryAdmitted.queued_jobs[0].media_job_public_id;
 expect(dryAdmitted.queued_jobs[0].dry_run).toBe(true);
 async function terminal(jobId){
  let result;
  for(let i=0;i<160;i++){
   result=required(await call("GET","/v1/media/jobs/"+jobId,undefined,headers),200,"worker result");
   if(!["queued","running","verifying"].includes(result.status))return result;
   await new Promise(resolve=>setTimeout(resolve,250));
  }
  throw Error("worker did not reach terminal state "+JSON.stringify(result));
 }
 const dryJob=await terminal(dryJobId);
 expect(dryJob.status).toBe("completed");
 expect(dryJob.dry_run).toBe(true);
 const dryPlan=required(await call("GET","/v1/media/jobs/"+dryJobId+"/operations",undefined,headers),200,"persisted dry-run plan");
 expect(dryPlan.operations.some(operation=>operation.operation_kind==="video_transcode")).toBe(true);
 expect(sourceHash()).toBe(beforeDryPlan);
 console.log("REAL_BROWSER_SAVE_RESTART_DRY_RUN_PLAN_PRESERVED_ORIGINAL "+dryJobId);
 await selector.selectOption(bound.media_discovery_association_public_id);
 await manual.getByLabel("Relative candidates").fill("Movies/original.mkv");
 await manual.getByRole("button",{name:"Preview candidates",exact:true}).click();
 await expect(manual).toContainText("Movies/original.mkv | Accepted");
 await manual.getByLabel("Queue jobs using the active profile").check();
 const admissionResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/discovery/runs");
 await manual.getByRole("button",{name:"Queue jobs",exact:true}).click();
 const admittedResponse=await admissionResponse;
 if(admittedResponse.status()!==201)throw Error("UI queue "+await admittedResponse.text());
 const admitted=await admittedResponse.json();
 await expect(manual.getByRole("status")).toHaveText("1 queued; 0 skipped.");
 if(admitted.queued_jobs.length!==1||admitted.queued_jobs[0].dry_run)throw Error("UI dry-run admission mismatch");
 const id=admitted.queued_jobs[0].media_job_public_id;
 const originalHash=execFileSync("docker",["exec",app,"sha256sum","/proof/source/Movies/original.mkv"],{encoding:"utf8"}).split(" ")[0];
 required(await call("POST","/v1/media/jobs/"+id+"/cancel",undefined,headers),204,"cancel first attempt");
 let job=await terminal(id);
 if(job.status!=="cancelled")throw Error("cancellation not observed "+JSON.stringify(job));
 const afterFailureHash=execFileSync("docker",["exec",app,"sha256sum","/proof/source/Movies/original.mkv"],{encoding:"utf8"}).split(" ")[0];
 if(afterFailureHash!==originalHash)throw Error("cancellation mutated source media");
 console.log("REAL_CANCELLED_ATTEMPT_PRESERVED_ORIGINAL "+JSON.stringify(job));
 await page.reload();
 const jobRow=page.locator('[data-job-id="'+id+'"]');
 await expect(jobRow).toBeVisible();
 await jobRow.locator("summary").click();
 const retry=jobRow.getByRole("button",{name:"Retry job",exact:true});
 await expect(retry).toBeDisabled();
 await jobRow.getByLabel("Confirm retry",{exact:true}).check();
 const retryResponse=page.waitForResponse(r=>r.request().method()==="POST"&&new URL(r.url()).pathname==="/v1/media/jobs/"+id+"/retry");
 await retry.click();
 const retried=await retryResponse;
 if(retried.status()!==204)throw Error("UI retry failed "+retried.status()+" "+await retried.text());
 job=await terminal(id);
 if(job.status!=="completed"||job.dry_run)throw Error("retried execution failed "+JSON.stringify(job));
 console.log("REAL_BROWSER_CONFIRMED_RETRY_COMPLETED "+id);
 const planned=required(await call("GET","/v1/media/jobs/"+id+"/operations",undefined,headers),200,"persisted plan");
 if(!planned.operations.some(operation=>operation.operation_kind==="video_transcode"))throw Error("missing transcode operation");

 const codec=execFileSync("docker",["exec",app,"ffprobe","-v","error","-select_streams","v:0","-show_entries","stream=codec_name","-of","default=nw=1:nk=1","/proof/source/Movies/original.mkv"],{encoding:"utf8"}).trim();
 if(codec!=="h264")throw Error("replacement is not H.264: "+codec);
 if(browserErrors.length)throw Error("browser errors: "+browserErrors.join("; "));
 console.log("EXECUTION_CHECKS "+JSON.stringify(required(await call("GET","/v1/media/jobs/"+id+"/verification-checks",undefined,headers),200,"execution checks")));
 console.log("REAL_BROWSER_EXECUTION_COMPLETED_AND_H264_REPLACEMENT "+id);
})().catch(async error=>{
 console.error(error);
 if(page){
  console.error("FAILED_PAGE_TITLE "+await page.title());
  console.error("BROWSER_ERRORS "+JSON.stringify(browserErrors));
  await page.screenshot({path:"target/operator-workflow-failure.png",fullPage:true});
 }
 process.exitCode=1;
}).finally(async()=>{
 if(browser)await browser.close();
 if(ui&&ui.exitCode===null){
  const stopped=new Promise(resolve=>ui.once("exit",resolve));ui.kill("SIGTERM");await stopped;
 }
 await fixtureRelay.close();
});
