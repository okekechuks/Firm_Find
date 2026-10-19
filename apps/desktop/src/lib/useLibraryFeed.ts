import { LibraryRevisionContext } from "./workspace-context";
import { useContext, useEffect, useState } from "react";
import { onLibraryChanged } from "./backend";

// Events are hints; views always read persisted state. Polling also covers missed events.
export function useLibraryFeed(desktop:boolean) {
  const shared=useContext(LibraryRevisionContext);
  const [revision,setRevision]=useState(0);
  useEffect(()=>{
    if(!desktop || shared!==null)return;
    let active=true;let unlisten:(()=>void)|undefined;let timer:ReturnType<typeof setTimeout>|undefined;
    const changed=()=>{if(timer)return;timer=setTimeout(()=>{timer=undefined;if(active)setRevision(v=>v+1);},200);};
    void onLibraryChanged(changed).then(fn=>{if(active)unlisten=fn;else fn();}).catch(()=>{/* Poll persisted records if live subscription is unavailable. */});
    const poll=setInterval(changed,1500);
    return()=>{active=false;unlisten?.();clearInterval(poll);clearTimeout(timer);};
  },[desktop,shared]);return shared??revision;
}
