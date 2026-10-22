import { LibraryRevisionContext } from "./workspace-context";
import { useContext, useEffect, useRef, useState } from "react";
import { onLibraryChanged } from "./backend";

// Events are hints; views always read persisted state. Polling also covers missed events.
export function useLibraryFeed(desktop:boolean, busy=false) {
  const shared=useContext(LibraryRevisionContext);
  const [revision,setRevision]=useState(0);
  const requestRefresh=useRef<()=>void>(()=>{});
  useEffect(()=>{
    if(!desktop || shared!==null)return;
    let active=true;let unlisten:(()=>void)|undefined;let timer:ReturnType<typeof setTimeout>|undefined;
    const changed=()=>{if(timer)return;timer=setTimeout(()=>{timer=undefined;if(active)setRevision(v=>v+1);},200);};
    requestRefresh.current=changed;
    void onLibraryChanged(changed).then(fn=>{if(active)unlisten=fn;else fn();}).catch(()=>{/* Poll persisted records if live subscription is unavailable. */});
    return()=>{active=false;requestRefresh.current=()=>{};unlisten?.();clearTimeout(timer);};
  },[desktop,shared]);
  // Poll cadence changes do not unregister a callback that native events may still target.
  useEffect(()=>{
    if(!desktop || shared!==null)return;
    const poll=setInterval(()=>requestRefresh.current(),busy?1500:30000);
    return()=>clearInterval(poll);
  },[desktop,shared,busy]);return shared??revision;
}
