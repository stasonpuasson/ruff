class Test(
    Aaaaaaaaaaaaaaaaa,
    Bbbbbbbbbbbbbbbb,
    DDDDDDDDDDDDDDDD,
    EEEEEEEEEEEEEE,
    metaclass=meta,
):
    pass


class Test((Aaaaaaaaaaaaaaaaa), Bbbbbbbbbbbbbbbb, metaclass=meta):
    pass

class Test( # trailing class comment
    Aaaaaaaaaaaaaaaaa, # trailing comment

    # in between comment

    Bbbbbbbbbbbbbbbb,
    # another leading comment
    DDDDDDDDDDDDDDDD,
    EEEEEEEEEEEEEE,
    # meta comment
    metaclass=meta, # trailing meta comment
):
    pass

class Test((Aaaa)):
    ...


class Test(aaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbb + cccccccccccccccccccccccc + dddddddddddddddddddddd + eeeeeeeee, ffffffffffffffffff, gggggggggggggggggg):
    pass

class Test(aaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbb * cccccccccccccccccccccccc + dddddddddddddddddddddd + eeeeeeeee, ffffffffffffffffff, gggggggggggggggggg):
    pass

class Test(Aaaa): # trailing comment
    pass
