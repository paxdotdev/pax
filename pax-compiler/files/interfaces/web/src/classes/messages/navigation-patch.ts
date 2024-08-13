export class NavigationPatch {
    public url?: string;
    public target?: string;

    fromPatch(jsonMessage: any) {
        this.url = jsonMessage["url"];
        this.target = jsonMessage["target"];
    }

    cleanUp(){
        this.url = undefined;
        this.target = undefined;
    }
}

