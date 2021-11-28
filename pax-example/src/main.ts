
@properties
class DeeperStruct {
    a: Property<number>;
    b: Property<string>;
}

@component
class Main {
    num_clicks : Property<number>;
    deeper_struct: Property<DeeperStruct>;

    incrementClicker(args: ClickArgs): void {
        this.num_clicks.set(this.num_clicks.get() + 1)
    }
}



////// lib //////
class Property<T> {
    val: T
    set(newVal: T): void {
        this.val = newVal;
    }
    get() : T {
        return this.val;
    }
}
[]